// Entry point for mont extension

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";
import { StringEnum } from "@mariozechner/pi-ai";
import { Text } from "@mariozechner/pi-tui";

const MONT_DOCS = `
## Mont Task Management

⚠️ CRITICAL: After every mont tool call, you will receive updated instructions via "mont prompt". You MUST read and follow these instructions carefully. They contain the current task status and required next steps.

Mont is a task management system for this project. Tasks are stored in .tasks/ directory.

### Key Concepts
- **Tasks**: Concrete work items with dependencies and gates
- **Jots**: Quick notes/ideas that can be distilled into tasks
- **Gates**: Checkpoints that must be passed before task completion (e.g., user-qa, test, semver)
- **Dependencies**: Tasks can depend on other tasks via "before" and "after" relationships

### Workflow
1. Use the mont tool to check current task status
2. Focus on the active task shown in status
3. Read the task file in .tasks/<task-id>.md for full details
4. Complete work, then pass gates as you finish each checkpoint
5. Do NOT modify .tasks/ files directly - use the mont tool
6. ALWAYS follow the instructions from mont prompt after each mont tool call

### Using the Mont Tool
The mont tool supports these actions:
- **status**: Check current task in progress
- **show <id>**: View task details
- **list**: List all incomplete tasks
- **ready**: Show tasks ready to work on
- **prompt**: Get full context for current task (IMPORTANT: follow its instructions)
- **stop**: Stop working on current task (makes it ready again)
- **jot**: Create a quick note (provide 'title')
- **create**: Create new task (provide 'content' with full markdown including frontmatter). Do NOT specify gates - defaults from config.yml apply automatically
- **edit <id>**: Update existing task (provide 'content' with full updated markdown)
- **patch <id>**: Update frontmatter fields (provide 'patch' with YAML)
- **append <id>**: Add text to description (provide 'text')
- **unlock <id>**: Mark gate as passed (provide 'gate' name)
- **distill <id>**: Convert jot to tasks (provide 'content' with task definitions). Do NOT specify gates - defaults from config.yml apply automatically

### Task Markdown Format
Tasks use YAML frontmatter:
\`\`\`markdown
---
id: my-task-id
title: Task Title
type: task
after:
  - dependency-task-id
---

## Description
Task description here.
\`\`\`

⚠️ IMPORTANT: Do NOT specify gates when creating or distilling tasks. Default gates are configured in .tasks/config.yml and will be applied automatically. Only specify gates if you need to override the defaults for a specific task.

### Gates
Gates are quality checkpoints. Common gates:
- user-qa: User has verified the changes work
- test: Tests pass
- architecture-validator: Architecture review passed
- semver: Version bump considered

### Reading Task Files
You CAN read .tasks/*.md files to understand task requirements.
You CANNOT write to .tasks/ directly - use the mont tool instead.
`;

interface MontResult {
	stdout: string;
	stderr: string;
	code: number;
	success: boolean;
}

export default function (pi: ExtensionAPI) {
	// After every mont tool call, inject mont prompt to keep agent aligned
	pi.on("tool_result", async (event, ctx) => {
		if (event.toolName !== "mont") return;

		// Update the task widget
		await updateTaskWidget(ctx);

		// Get fresh mont prompt
		const promptResult = await runMont(["prompt"], ctx.cwd);
		const cleanPrompt = promptResult.stdout.replace(/\x1b\[[0-9;]*m/g, "").trim();

		if (cleanPrompt) {
			pi.sendMessage(
				{
					customType: "mont-prompt-update",
					content: `⚠️ IMPORTANT - CURRENT TASK STATUS (you must follow these instructions):\n\n${cleanPrompt}`,
					display: false,
				},
				{ deliverAs: "followUp", triggerTurn: false }
			);
		}
	});

	// Block writes to .tasks directory
	pi.on("tool_call", async (event) => {
		if (event.toolName === "write" || event.toolName === "edit") {
			const path = event.input.path as string;
			if (path && (path.startsWith(".tasks/") || path.startsWith(".tasks\\") || path === ".tasks")) {
				return {
					block: true,
					reason: "Writing to .tasks/ is not allowed. Use mont commands to manage tasks.",
				};
			}
		}

		if (event.toolName === "bash") {
			const command = event.input.command as string;
			// Block common write patterns to .tasks
			if (command && /\b(rm|mv|cp|touch|mkdir|rmdir|echo\s.*>|cat\s.*>|tee)\b.*\.tasks/.test(command)) {
				return {
					block: true,
					reason: "Modifying .tasks/ via bash is not allowed. Use mont commands to manage tasks.",
				};
			}
		}
	});

	/**
	 * Execute a mont command and capture output
	 */
	async function runMont(args: string[], cwd?: string, signal?: AbortSignal): Promise<MontResult> {
		const result = await pi.exec("mont", args, { cwd, signal });
		return {
			stdout: result.stdout,
			stderr: result.stderr,
			code: result.code,
			success: result.code === 0,
		};
	}

	/**
	 * Execute a mont command with stdin input
	 */
	async function runMontWithStdin(args: string[], stdin: string, cwd?: string, signal?: AbortSignal): Promise<MontResult> {
		// Use bash to pipe content to mont via stdin
		// Escape the content for shell safety using base64
		const base64Content = Buffer.from(stdin).toString("base64");
		const command = `echo "${base64Content}" | base64 -d | mont ${args.map((a) => `'${a.replace(/'/g, "'\\''")}'`).join(" ")}`;
		const result = await pi.exec("bash", ["-c", command], { cwd, signal });
		return {
			stdout: result.stdout,
			stderr: result.stderr,
			code: result.code,
			success: result.code === 0,
		};
	}

	// /mont-status command - show status of in-progress tasks
	pi.registerCommand("mont-status", {
		description: "Show status of in-progress mont tasks",
		handler: async (_args, ctx) => {
			const result = await runMont(["status"], ctx.cwd);

			if (result.success) {
				if (result.stdout.trim()) {
					ctx.ui.notify(result.stdout.trim(), "info");
				} else {
					ctx.ui.notify("No tasks in progress", "info");
				}
			} else {
				ctx.ui.notify(result.stderr.trim() || "mont status failed", "error");
			}
		},
	});

	// /mont-list command - list all tasks in the task graph
	pi.registerCommand("mont-list", {
		description: "List all mont tasks in the task graph",
		handler: async (args, ctx) => {
			const montArgs = ["list"];
			if (args?.includes("--show-completed")) {
				montArgs.push("--show-completed");
			}
			const result = await runMont(montArgs, ctx.cwd);

			if (result.success) {
				if (result.stdout.trim()) {
					ctx.ui.notify(result.stdout.trim(), "info");
				} else {
					ctx.ui.notify("No tasks found", "info");
				}
			} else {
				ctx.ui.notify(result.stderr.trim() || "mont list failed", "error");
			}
		},
	});

	// /mont-ready command - show tasks ready to work on
	pi.registerCommand("mont-ready", {
		description: "Show mont tasks ready to work on",
		handler: async (_args, ctx) => {
			const result = await runMont(["ready"], ctx.cwd);

			if (result.success) {
				if (result.stdout.trim()) {
					ctx.ui.notify(result.stdout.trim(), "info");
				} else {
					ctx.ui.notify("No tasks ready", "info");
				}
			} else {
				ctx.ui.notify(result.stderr.trim() || "mont ready failed", "error");
			}
		},
	});

	/**
	 * Parse mont ready output to extract task IDs and titles
	 * Format: "◇ [type]   task-id  Title text" or "◉ [type]   task-id  Title text"
	 */
	function parseReadyTasks(output: string): Array<{ id: string; type: string; title: string }> {
		const tasks: Array<{ id: string; type: string; title: string }> = [];
		const lines = output.split("\n");

		for (const line of lines) {
			// Match lines like: ◇ [jot]   crisp-titmouse  Test task 2
			// or: ◉ [task]  task-id  Title text
			// The line contains ANSI codes, so we strip them first
			const stripped = line.replace(/\x1b\[[0-9;]*m/g, "");
			const match = stripped.match(/^[◇◉]\s+\[(\w+)\]\s+(\S+)\s+(.*)$/);
			if (match) {
				tasks.push({
					type: match[1],
					id: match[2],
					title: match[3].trim(),
				});
			}
		}

		return tasks;
	}

	// /mont-start command - select and start a task, then prompt agent to work on it
	pi.registerCommand("mont-start", {
		description: "Select and start a mont task",
		handler: withWidgetUpdate(async (_args, ctx) => {
			// Get ready tasks
			const readyResult = await runMont(["ready"], ctx.cwd);

			if (!readyResult.success) {
				ctx.ui.notify(readyResult.stderr.trim() || "mont ready failed", "error");
				return;
			}

			const tasks = parseReadyTasks(readyResult.stdout);

			if (tasks.length === 0) {
				ctx.ui.notify("No tasks ready to start", "info");
				return;
			}

			// Let user select a task
			const options = tasks.map((t) => `[${t.type}] ${t.id} - ${t.title}`);
			const selected = await ctx.ui.select("Select task to start:", options);

			if (!selected) {
				return; // User cancelled
			}

			// Extract task ID from selection
			const selectedTask = tasks[options.indexOf(selected)];

			// Start the task
			const startResult = await runMont(["start", selectedTask.id], ctx.cwd);

			if (!startResult.success) {
				ctx.ui.notify(startResult.stderr.trim() || "mont start failed", "error");
				return;
			}

			ctx.ui.notify(`Started task: ${selectedTask.id}`, "info");

			// Get the task prompt and inject into context (hidden from UI)
			const promptResult = await runMont(["prompt"], ctx.cwd);
			const cleanPrompt = promptResult.stdout.replace(/\x1b\[[0-9;]*m/g, "").trim();

			if (cleanPrompt) {
				// Inject task context without displaying
				pi.sendMessage(
					{
						customType: "mont-task-context",
						content: cleanPrompt,
						display: false,
					},
					{ triggerTurn: false }
				);

				// Send a short user message to trigger the agent
				pi.sendUserMessage(`Start working on task: ${selectedTask.id}`);
			}
		}),
	});

	/**
	 * Parse mont status output to extract the active task ID
	 */
	function parseActiveTaskId(output: string): string | null {
		const stripped = output.replace(/\x1b\[[0-9;]*m/g, "");
		// Match "Id" line followed by task ID
		const match = stripped.match(/Id\s+(\S+)/);
		return match ? match[1] : null;
	}

	interface GateInfo {
		name: string;
		status: "pending" | "passed" | "skipped";
	}

	/**
	 * Parse mont status output to extract gates for the active task
	 * Pending gates: red (31m) bullet, white (37m) name
	 * Passed gates: green (92m) checkmark, gray (90m) name
	 * Skipped gates: yellow bullet, gray name
	 */
	function parseActiveTaskGates(output: string): GateInfo[] {
		const gates: GateInfo[] = [];

		// Find the Gates section
		const gatesMatch = output.match(/Gates\s*\x1b\[0m([\s\S]*?)(?:\n\n|\n\x1b\[1mInfo|$)/);
		if (!gatesMatch) return gates;

		const gatesSection = gatesMatch[1];
		const lines = gatesSection.split("\n");

		for (const line of lines) {
			// Check for green (92m) = passed
			if (line.includes("\x1b[92m")) {
				const nameMatch = line.match(/\x1b\[90m(\S+)/);
				if (nameMatch) {
					gates.push({ name: nameMatch[1], status: "passed" });
				}
			}
			// Check for red (31m) = pending
			else if (line.includes("\x1b[31m")) {
				const nameMatch = line.match(/\x1b\[37m(\S+)/);
				if (nameMatch) {
					gates.push({ name: nameMatch[1], status: "pending" });
				}
			}
			// Check for yellow (33m) = skipped
			else if (line.includes("\x1b[33m")) {
				const nameMatch = line.match(/\x1b\[90m(\S+)/);
				if (nameMatch) {
					gates.push({ name: nameMatch[1], status: "skipped" });
				}
			}
		}

		return gates;
	}

	// Register mont tool for LLM to query task information
	pi.registerTool({
		name: "mont",
		label: "Mont",
		description: `Query mont task management system. Actions:
- status: Show current task in progress with gates
- show <id>: Show details for a specific task
- list: List all incomplete tasks
- ready: Show tasks ready to work on
- prompt: Generate context prompt for current task
- jot <title>: Create a quick jot (idea/note)
- create: Create a new task (provide 'content' with full task markdown including frontmatter)
- edit <id>: Edit existing task (provide 'content' with full updated task markdown)
- patch <id>: Update task frontmatter fields (provide 'patch' with YAML like "title: New Title")
- append <id>: Append text to task description (provide 'text' to append)
- unlock <id>: Mark a gate as passed (provide 'gate' name)
- distill <id>: Convert a jot to tasks (provide 'content' with task definitions separated by ---)
- stop: Stop working on current task (makes it ready again)
- delete <id>: Delete a task and remove all references to it`,
		parameters: Type.Object({
			action: StringEnum([
				"status",
				"show",
				"list",
				"ready",
				"prompt",
				"jot",
				"create",
				"edit",
				"patch",
				"append",
				"unlock",
				"distill",
				"stop",
				"delete",
			] as const),
			id: Type.Optional(Type.String({ description: "Task ID (required for show, edit, patch, append, unlock, distill)" })),
			title: Type.Optional(Type.String({ description: "Title for jot action" })),
			content: Type.Optional(Type.String({ description: "Full task markdown content for create/edit/distill (include YAML frontmatter)" })),
			patch: Type.Optional(Type.String({ description: "YAML patch for patch action (e.g., 'title: New Title')" })),
			text: Type.Optional(Type.String({ description: "Text to append for append action" })),
			gate: Type.Optional(Type.String({ description: "Gate name for unlock action" })),
		}),

		async execute(_toolCallId, params, _onUpdate, ctx, signal) {
			const { action, id, title, content, patch, text, gate } = params as {
				action: string;
				id?: string;
				title?: string;
				content?: string;
				patch?: string;
				text?: string;
				gate?: string;
			};

			let result: MontResult;

			switch (action) {
				case "status":
					result = await runMont(["status"], ctx.cwd, signal);
					break;

				case "show":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'show' action" }],
							details: { error: "missing id" },
						};
					}
					result = await runMont(["show", id], ctx.cwd, signal);
					break;

				case "list":
					result = await runMont(["list"], ctx.cwd, signal);
					break;

				case "ready":
					result = await runMont(["ready"], ctx.cwd, signal);
					break;

				case "prompt":
					result = await runMont(["prompt"], ctx.cwd, signal);
					break;

				case "jot":
					if (!title) {
						return {
							content: [{ type: "text", text: "Error: 'title' parameter required for 'jot' action" }],
							details: { error: "missing title" },
						};
					}
					result = await runMont(["jot", "-q", title], ctx.cwd, signal);
					break;

				case "create":
					if (!content) {
						return {
							content: [{ type: "text", text: "Error: 'content' parameter required for 'create' action" }],
							details: { error: "missing content" },
						};
					}
					result = await runMontWithStdin(["task", "--stdin"], content, ctx.cwd, signal);
					break;

				case "edit":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'edit' action" }],
							details: { error: "missing id" },
						};
					}
					if (!content) {
						return {
							content: [{ type: "text", text: "Error: 'content' parameter required for 'edit' action" }],
							details: { error: "missing content" },
						};
					}
					result = await runMontWithStdin(["task", id, "--stdin"], content, ctx.cwd, signal);
					break;

				case "patch":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'patch' action" }],
							details: { error: "missing id" },
						};
					}
					if (!patch) {
						return {
							content: [{ type: "text", text: "Error: 'patch' parameter required for 'patch' action" }],
							details: { error: "missing patch" },
						};
					}
					result = await runMont(["task", id, "--patch", patch], ctx.cwd, signal);
					break;

				case "append":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'append' action" }],
							details: { error: "missing id" },
						};
					}
					if (!text) {
						return {
							content: [{ type: "text", text: "Error: 'text' parameter required for 'append' action" }],
							details: { error: "missing text" },
						};
					}
					result = await runMont(["task", id, "--append", text], ctx.cwd, signal);
					break;

				case "unlock":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'unlock' action" }],
							details: { error: "missing id" },
						};
					}
					if (!gate) {
						return {
							content: [{ type: "text", text: "Error: 'gate' parameter required for 'unlock' action" }],
							details: { error: "missing gate" },
						};
					}
					result = await runMont(["unlock", id, "--passed", gate], ctx.cwd, signal);
					break;

				case "distill":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'distill' action" }],
							details: { error: "missing id" },
						};
					}
					if (!content) {
						return {
							content: [{ type: "text", text: "Error: 'content' parameter required for 'distill' action" }],
							details: { error: "missing content" },
						};
					}
					result = await runMontWithStdin(["distill", id, "--stdin"], content, ctx.cwd, signal);
					break;

				case "stop":
					// ID is optional - defaults to current in-progress task
					result = id
						? await runMont(["stop", id], ctx.cwd, signal)
						: await runMont(["stop"], ctx.cwd, signal);
					break;

				case "delete":
					if (!id) {
						return {
							content: [{ type: "text", text: "Error: 'id' parameter required for 'delete' action" }],
							details: { error: "missing id" },
						};
					}
					result = await runMont(["delete", id, "--force"], ctx.cwd, signal);
					break;

				default:
					return {
						content: [{ type: "text", text: `Unknown action: ${action}` }],
						details: { error: "unknown action" },
					};
			}

			// Strip ANSI codes for cleaner LLM output
			const cleanOutput = (result.stdout + result.stderr).replace(/\x1b\[[0-9;]*m/g, "").trim();

			return {
				content: [{ type: "text", text: cleanOutput || `No output from mont ${action}` }],
				details: { action, id, title, gate, success: result.success, code: result.code },
			};
		},

		renderCall(args, theme) {
			const action = args.action as string;
			let text = theme.fg("toolTitle", theme.bold("mont "));
			text += theme.fg("accent", action);

			// Add relevant context based on action
			if (args.id) {
				text += theme.fg("muted", " → ") + theme.fg("warning", args.id as string);
			}
			if (args.title) {
				text += theme.fg("muted", ` "${args.title as string}"`);
			}
			if (args.gate) {
				text += theme.fg("muted", " gate:") + theme.fg("success", args.gate as string);
			}
			if (args.patch) {
				text += theme.fg("dim", ` [patch]`);
			}
			if (args.content) {
				const lines = (args.content as string).split("\n").length;
				text += theme.fg("dim", ` [${lines} lines]`);
			}
			if (args.text) {
				const preview = (args.text as string).slice(0, 30);
				text += theme.fg("dim", ` "${preview}${(args.text as string).length > 30 ? "..." : ""}"`);
			}

			return new Text(text, 0, 0);
		},

		renderResult(result, { expanded }, theme) {
			const details = result.details as {
				action?: string;
				id?: string;
				title?: string;
				gate?: string;
				success?: boolean;
				code?: number;
				error?: string;
			} | undefined;

			if (!details) {
				const text = result.content[0];
				return new Text(text?.type === "text" ? text.text : "", 0, 0);
			}

			if (details.error) {
				return new Text(theme.fg("error", `✗ Error: ${details.error}`), 0, 0);
			}

			if (!details.success) {
				const text = result.content[0];
				const output = text?.type === "text" ? text.text : "Command failed";
				return new Text(theme.fg("error", "✗ ") + theme.fg("muted", output), 0, 0);
			}

			const action = details.action || "unknown";
			let output = "";

			switch (action) {
				case "status":
					output = theme.fg("success", "✓ ") + theme.fg("muted", "Task status retrieved");
					break;
				case "show":
					output = theme.fg("success", "✓ ") + theme.fg("muted", `Showing task ${details.id || ""}`);
					break;
				case "list":
					output = theme.fg("success", "✓ ") + theme.fg("muted", "Task list retrieved");
					break;
				case "ready":
					output = theme.fg("success", "✓ ") + theme.fg("muted", "Ready tasks retrieved");
					break;
				case "prompt":
					output = theme.fg("success", "✓ ") + theme.fg("muted", "Task prompt generated");
					break;
				case "jot":
					output = theme.fg("success", "✓ Created jot: ") + theme.fg("accent", details.title || "");
					break;
				case "create":
					output = theme.fg("success", "✓ ") + theme.fg("muted", "Task created");
					break;
				case "edit":
					output = theme.fg("success", "✓ Updated: ") + theme.fg("accent", details.id || "");
					break;
				case "patch":
					output = theme.fg("success", "✓ Patched: ") + theme.fg("accent", details.id || "");
					break;
				case "append":
					output = theme.fg("success", "✓ Appended to: ") + theme.fg("accent", details.id || "");
					break;
				case "unlock":
					output = theme.fg("success", "✓ Unlocked gate ") + theme.fg("warning", details.gate || "") + theme.fg("muted", " on ") + theme.fg("accent", details.id || "");
					break;
				case "distill":
					output = theme.fg("success", "✓ Distilled: ") + theme.fg("accent", details.id || "");
					break;
				case "stop":
					output = theme.fg("success", "✓ Stopped task") + (details.id ? ": " + theme.fg("accent", details.id) : "");
					break;
				case "delete":
					output = theme.fg("success", "✓ Deleted: ") + theme.fg("accent", details.id || "");
					break;
				default:
					output = theme.fg("success", "✓ ") + theme.fg("muted", `${action} completed`);
			}

			// Show full output if expanded
			if (expanded) {
				const text = result.content[0];
				const fullOutput = text?.type === "text" ? text.text : "";
				if (fullOutput) {
					output += "\n" + theme.fg("dim", fullOutput);
				}
			}

			return new Text(output, 0, 0);
		},
	});

	// Track whether we've injected mont context this session
	let contextInjected = false;
	let lastKnownTaskId: string | null = null;

	// Type for command handler context
	type CommandContext = { cwd: string; hasUI: boolean; ui: any };

	/**
	 * Wrapper that updates the task widget after a command handler runs
	 */
	function withWidgetUpdate<T extends (args: string | undefined, ctx: CommandContext) => Promise<void>>(
		handler: T
	): T {
		return (async (args: string | undefined, ctx: CommandContext) => {
			await handler(args, ctx);
			await updateTaskWidget(ctx);
		}) as T;
	}

	/**
	 * Update the task status widget
	 */
	async function updateTaskWidget(ctx: CommandContext) {
		if (!ctx.hasUI) return;

		const statusResult = await runMont(["status"], ctx.cwd);

		if (!statusResult.success) {
			// No task graph - clear widget
			ctx.ui.setWidget("mont-task", undefined);
			return;
		}

		const taskId = parseActiveTaskId(statusResult.stdout);

		if (!taskId) {
			ctx.ui.setWidget("mont-task", (tui: any, theme: any) => {
				const text = theme.fg("dim", "mont: ") + theme.fg("muted", "no active task");
				return new Text(text, 0, 0);
			});
			return;
		}

		// Parse gates from status
		const gates = parseActiveTaskGates(statusResult.stdout);
		const passedCount = gates.filter((g) => g.status === "passed").length;
		const totalGates = gates.length;

		// Parse title from status
		const stripped = statusResult.stdout.replace(/\x1b\[[0-9;]*m/g, "");
		const titleMatch = stripped.match(/Title\s+(.+)/);
		const title = titleMatch ? titleMatch[1].trim() : taskId;

		ctx.ui.setWidget("mont-task", (tui: any, theme: any) => {
			let text = theme.fg("dim", "mont: ");
			text += theme.fg("accent", taskId);
			text += theme.fg("dim", " - ");
			text += theme.fg("muted", title.length > 40 ? title.slice(0, 40) + "..." : title);

			if (totalGates > 0) {
				text += theme.fg("dim", " [");
				text += passedCount === totalGates
					? theme.fg("success", `${passedCount}/${totalGates}`)
					: theme.fg("warning", `${passedCount}/${totalGates}`);
				text += theme.fg("dim", " gates]");
			}

			return new Text(text, 0, 0);
		});
	}

	// Inject mont docs and prompt once at session start
	pi.on("session_start", async (_event, ctx) => {
		contextInjected = false;
		lastKnownTaskId = null;

		// Check for active task
		const statusResult = await runMont(["status"], ctx.cwd);
		if (statusResult.success) {
			lastKnownTaskId = parseActiveTaskId(statusResult.stdout);
		}

		// Update task widget
		await updateTaskWidget(ctx);

		// Inject mont documentation as a one-time message
		pi.sendMessage(
			{
				customType: "mont-context",
				content: MONT_DOCS,
				display: false, // Don't clutter the UI
			},
			{ deliverAs: "nextTurn", triggerTurn: false }
		);
		contextInjected = true;

		// Always inject mont prompt (it handles both active task and no-task states)
		const promptResult = await runMont(["prompt"], ctx.cwd);
		const cleanPrompt = promptResult.stdout.replace(/\x1b\[[0-9;]*m/g, "").trim();
		if (cleanPrompt) {
			pi.sendMessage(
				{
					customType: "mont-task-context",
					content: cleanPrompt,
					display: false,
				},
				{ deliverAs: "nextTurn", triggerTurn: false }
			);
		}
	});

	// Re-inject after compaction (context is lost)
	pi.on("session_compact", async (_event, ctx) => {
		const statusResult = await runMont(["status"], ctx.cwd);
		const activeTaskId = statusResult.success ? parseActiveTaskId(statusResult.stdout) : null;

		// Re-inject docs after compaction
		pi.sendMessage(
			{
				customType: "mont-context",
				content: MONT_DOCS,
				display: false,
			},
			{ deliverAs: "nextTurn", triggerTurn: false }
		);

		// Re-inject active task context if any
		if (activeTaskId) {
			const promptResult = await runMont(["prompt"], ctx.cwd);
			const cleanPrompt = promptResult.stdout.replace(/\x1b\[[0-9;]*m/g, "").trim();
			if (cleanPrompt) {
				pi.sendMessage(
					{
						customType: "mont-task-context",
						content: `## Active Task\n\n${cleanPrompt}`,
						display: false,
					},
					{ deliverAs: "nextTurn", triggerTurn: false }
				);
			}
		}
	});

	// /mont-task command - create or edit a task using pi's editor
	pi.registerCommand("mont-task", {
		description: "Create or edit a mont task",
		handler: withWidgetUpdate(async (args, ctx) => {
			const taskId = args?.trim();

			let initialContent = "";
			let isNew = false;

			if (taskId) {
				// Editing existing task - read current content
				const showResult = await runMont(["show", taskId], ctx.cwd);
				if (!showResult.success) {
					ctx.ui.notify(showResult.stderr.trim() || `Task ${taskId} not found`, "error");
					return;
				}

				// Read the actual task file
				const fs = await import("node:fs/promises");
				const path = await import("node:path");
				const taskPath = path.join(ctx.cwd, ".tasks", `${taskId}.md`);
				try {
					initialContent = await fs.readFile(taskPath, "utf-8");
				} catch {
					ctx.ui.notify(`Could not read task file: ${taskPath}`, "error");
					return;
				}
			} else {
				// New task - provide template
				isNew = true;
				initialContent = `---
id: new-task-id
title: Task Title
type: task
---

## Description

Describe the task here.
`;
			}

			// Open pi's editor
			const edited = await ctx.ui.editor(isNew ? "Create new task:" : `Edit task ${taskId}:`, initialContent);

			if (!edited || edited === initialContent) {
				ctx.ui.notify("No changes made", "info");
				return;
			}

			// Save via mont --stdin
			const saveArgs = taskId ? ["task", taskId, "--stdin"] : ["task", "--stdin"];
			const saveResult = await runMontWithStdin(saveArgs, edited, ctx.cwd);

			if (saveResult.success) {
				ctx.ui.notify(isNew ? "Task created" : `Task ${taskId} updated`, "info");
			} else {
				ctx.ui.notify(saveResult.stderr.trim() || "Failed to save task", "error");
			}
		}),
	});

	// /mont-jot command - create a jot using pi's editor
	pi.registerCommand("mont-jot", {
		description: "Create a mont jot",
		handler: withWidgetUpdate(async (args, ctx) => {
			const title = args?.trim();

			// If title provided, use quick mode
			if (title) {
				const result = await runMont(["jot", "-q", title], ctx.cwd);
				if (result.success) {
					ctx.ui.notify(`Jot created: ${title}`, "info");
				} else {
					ctx.ui.notify(result.stderr.trim() || "Failed to create jot", "error");
				}
				return;
			}

			// No title - open editor with template
			const initialContent = `---
id: 
title: 
type: jot
---

`;

			const edited = await ctx.ui.editor("Create new jot:", initialContent);

			if (!edited || edited === initialContent) {
				ctx.ui.notify("No changes made", "info");
				return;
			}

			const saveResult = await runMontWithStdin(["task", "--stdin"], edited, ctx.cwd);

			if (saveResult.success) {
				ctx.ui.notify("Jot created", "info");
			} else {
				ctx.ui.notify(saveResult.stderr.trim() || "Failed to create jot", "error");
			}
		}),
	});

	// /mont-distill command - convert a jot to tasks using pi's editor
	pi.registerCommand("mont-distill", {
		description: "Convert a mont jot into tasks",
		handler: withWidgetUpdate(async (args, ctx) => {
			let jotId = args?.trim();

			// If no ID provided, let user select from jots
			if (!jotId) {
				const listResult = await runMont(["list"], ctx.cwd);
				if (!listResult.success) {
					ctx.ui.notify(listResult.stderr.trim() || "mont list failed", "error");
					return;
				}

				const tasks = parseReadyTasks(listResult.stdout);
				const jots = tasks.filter((t) => t.type === "jot");

				if (jots.length === 0) {
					ctx.ui.notify("No jots to distill", "info");
					return;
				}

				const options = jots.map((t) => `${t.id} - ${t.title}`);
				const selected = await ctx.ui.select("Select jot to distill:", options);

				if (!selected) {
					return;
				}

				jotId = jots[options.indexOf(selected)].id;
			}

			// Read the jot content for context
			const fs = await import("node:fs/promises");
			const path = await import("node:path");
			const jotPath = path.join(ctx.cwd, ".tasks", `${jotId}.md`);
			let jotContent = "";
			try {
				jotContent = await fs.readFile(jotPath, "utf-8");
			} catch {
				// Continue without jot content
			}

			// Provide template for distilled tasks
			const initialContent = `# Distilling: ${jotId}
# Original jot content is shown below for reference.
# Delete these comment lines and define your tasks.
# Separate multiple tasks with ---

---
id: task-1
title: First Task
type: task
---

Description of first task.

---
id: task-2
title: Second Task  
type: task
after:
  - task-1
---

Description of second task.

# --- Original Jot ---
# ${jotContent.split("\n").join("\n# ")}
`;

			const edited = await ctx.ui.editor(`Distill jot ${jotId} into tasks:`, initialContent);

			if (!edited) {
				ctx.ui.notify("Cancelled", "info");
				return;
			}

			// Remove comment lines and clean up
			const cleanedContent = edited
				.split("\n")
				.filter((line: string) => !line.startsWith("#"))
				.join("\n")
				.trim();

			if (!cleanedContent) {
				ctx.ui.notify("No task content provided", "info");
				return;
			}

			const distillResult = await runMontWithStdin(["distill", jotId, "--stdin"], cleanedContent, ctx.cwd);

			if (distillResult.success) {
				ctx.ui.notify(`Distilled ${jotId} into tasks`, "info");
			} else {
				ctx.ui.notify(distillResult.stderr.trim() || "Failed to distill jot", "error");
			}
		}),
	});

	// /mont-stop command - stop working on current task
	pi.registerCommand("mont-stop", {
		description: "Stop working on the current mont task",
		handler: withWidgetUpdate(async (args, ctx) => {
			const taskId = args?.trim();

			const result = taskId
				? await runMont(["stop", taskId], ctx.cwd)
				: await runMont(["stop"], ctx.cwd);

			if (result.success) {
				const cleanOutput = result.stdout.replace(/\x1b\[[0-9;]*m/g, "").trim();
				ctx.ui.notify(cleanOutput || "Task stopped", "info");
			} else {
				ctx.ui.notify(result.stderr.trim() || "mont stop failed", "error");
			}
		}),
	});

	// /mont-show command - show details for a task
	pi.registerCommand("mont-show", {
		description: "Show details for a mont task",
		handler: async (args, ctx) => {
			// If an ID was provided as argument, use it directly
			if (args?.trim()) {
				const result = await runMont(["show", args.trim()], ctx.cwd);
				if (result.success) {
					ctx.ui.notify(result.stdout.trim(), "info");
				} else {
					ctx.ui.notify(result.stderr.trim() || "mont show failed", "error");
				}
				return;
			}

			// Otherwise, let user select from list
			const listResult = await runMont(["list"], ctx.cwd);

			if (!listResult.success) {
				ctx.ui.notify(listResult.stderr.trim() || "mont list failed", "error");
				return;
			}

			const tasks = parseReadyTasks(listResult.stdout);

			if (tasks.length === 0) {
				ctx.ui.notify("No tasks found", "info");
				return;
			}

			// Let user select a task
			const options = tasks.map((t) => `[${t.type}] ${t.id} - ${t.title}`);
			const selected = await ctx.ui.select("Select task to show:", options);

			if (!selected) {
				return; // User cancelled
			}

			const selectedTask = tasks[options.indexOf(selected)];
			const result = await runMont(["show", selectedTask.id], ctx.cwd);

			if (result.success) {
				ctx.ui.notify(result.stdout.trim(), "info");
			} else {
				ctx.ui.notify(result.stderr.trim() || "mont show failed", "error");
			}
		},
	});

	// /mont-delete command - delete a task
	pi.registerCommand("mont-delete", {
		description: "Delete a mont task",
		handler: withWidgetUpdate(async (args, ctx) => {
			let taskId = args?.trim();

			// If no ID provided, let user select from incomplete tasks
			if (!taskId) {
				const listResult = await runMont(["list"], ctx.cwd);
				if (!listResult.success) {
					ctx.ui.notify(listResult.stderr.trim() || "mont list failed", "error");
					return;
				}

				const tasks = parseReadyTasks(listResult.stdout);

				if (tasks.length === 0) {
					ctx.ui.notify("No tasks to delete", "info");
					return;
				}

				const options = tasks.map((t) => `[${t.type}] ${t.id} - ${t.title}`);
				const selected = await ctx.ui.select("Select task to delete:", options);

				if (!selected) {
					return;
				}

				taskId = tasks[options.indexOf(selected)].id;
			}

			// Confirm deletion
			const confirmed = await ctx.ui.confirm("Delete task?", `Are you sure you want to delete "${taskId}"?`);

			if (!confirmed) {
				ctx.ui.notify("Cancelled", "info");
				return;
			}

			const deleteResult = await runMont(["delete", taskId, "--force"], ctx.cwd);

			if (deleteResult.success) {
				ctx.ui.notify(`Deleted task: ${taskId}`, "info");
			} else {
				ctx.ui.notify(deleteResult.stderr.trim() || "Failed to delete task", "error");
			}
		}),
	});

	// /mont-lock command - reset gates back to pending
	pi.registerCommand("mont-lock", {
		description: "Reset mont task gates back to pending",
		handler: withWidgetUpdate(async (_args, ctx) => {
			// Get active task from status
			const statusResult = await runMont(["status"], ctx.cwd);

			if (!statusResult.success) {
				ctx.ui.notify(statusResult.stderr.trim() || "mont status failed", "error");
				return;
			}

			const activeTaskId = parseActiveTaskId(statusResult.stdout);

			if (!activeTaskId) {
				ctx.ui.notify("No active task. Start a task first with /mont-start", "info");
				return;
			}

			const gates = parseActiveTaskGates(statusResult.stdout);
			const passedGates = gates.filter((g) => g.status === "passed");

			if (passedGates.length === 0) {
				ctx.ui.notify(`No passed gates to lock on task ${activeTaskId}`, "info");
				return;
			}

			// Let user select gates to lock (only show passed gates)
			const options = passedGates.map((g) => g.name);
			const selected = await ctx.ui.select("Select gate to lock:", options);

			if (!selected) {
				return; // User cancelled
			}

			// Lock the gate
			const lockResult = await runMont(["lock", activeTaskId, "--gates", selected], ctx.cwd);

			if (lockResult.success) {
				ctx.ui.notify(`Locked gate "${selected}" on ${activeTaskId}`, "info");
			} else {
				ctx.ui.notify(lockResult.stderr.trim() || "mont lock failed", "error");
			}
		}),
	});
}
