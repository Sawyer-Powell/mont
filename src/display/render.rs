use super::layout::Cell;

/// Convert a Connection cell to its ASCII symbol representation.
///
/// Returns a 2-char string (symbol + space) to maintain column alignment.
/// Uses rounded corners for a softer appearance.
///
/// Symbol mapping:
/// ```text
/// {up, down}              -> │
/// {left, right}           -> ─
/// {up, down, right}       -> ├
/// {up, down, left}        -> ┤
/// {down, right}           -> ╭
/// {down, left}            -> ╮
/// {up, right}             -> ╰
/// {up, left}              -> ╯
/// {up, down, left, right} -> ┼
/// ```
pub fn connection_symbol(cell: &Cell) -> &'static str {
    match cell {
        Cell::Connection {
            up,
            down,
            left,
            right,
        } => match (*up, *down, *left, *right) {
            // Vertical
            (true, true, false, false) => "│ ",
            // Horizontal
            (false, false, true, true) => "──",
            // T-junctions
            (true, true, false, true) => "├─",
            (true, true, true, false) => "┤ ",
            (false, true, true, true) => "┬─",
            (true, false, true, true) => "┴─",
            // Corners (rounded)
            (false, true, false, true) => "╭─",
            (false, true, true, false) => "╮ ",
            (true, false, false, true) => "╰─",
            (true, false, true, false) => "╯ ",
            // Cross
            (true, true, true, true) => "┼─",
            // Half-lines (endpoints)
            (true, false, false, false) => "╵ ",
            (false, true, false, false) => "╷ ",
            (false, false, true, false) => "╴ ",
            (false, false, false, true) => "╶─",
            // Empty connection
            (false, false, false, false) => "  ",
        },
        Cell::Task(_) | Cell::Empty => "  ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(up: bool, down: bool, left: bool, right: bool) -> Cell {
        Cell::Connection {
            up,
            down,
            left,
            right,
        }
    }

    #[test]
    fn test_vertical_line() {
        assert_eq!(connection_symbol(&conn(true, true, false, false)), "│ ");
    }

    #[test]
    fn test_horizontal_line() {
        assert_eq!(connection_symbol(&conn(false, false, true, true)), "──");
    }

    #[test]
    fn test_t_junctions() {
        assert_eq!(connection_symbol(&conn(true, true, false, true)), "├─");
        assert_eq!(connection_symbol(&conn(true, true, true, false)), "┤ ");
        assert_eq!(connection_symbol(&conn(false, true, true, true)), "┬─");
        assert_eq!(connection_symbol(&conn(true, false, true, true)), "┴─");
    }

    #[test]
    fn test_rounded_corners() {
        assert_eq!(connection_symbol(&conn(false, true, false, true)), "╭─");
        assert_eq!(connection_symbol(&conn(false, true, true, false)), "╮ ");
        assert_eq!(connection_symbol(&conn(true, false, false, true)), "╰─");
        assert_eq!(connection_symbol(&conn(true, false, true, false)), "╯ ");
    }

    #[test]
    fn test_cross() {
        assert_eq!(connection_symbol(&conn(true, true, true, true)), "┼─");
    }

    #[test]
    fn test_endpoints() {
        assert_eq!(connection_symbol(&conn(true, false, false, false)), "╵ ");
        assert_eq!(connection_symbol(&conn(false, true, false, false)), "╷ ");
        assert_eq!(connection_symbol(&conn(false, false, true, false)), "╴ ");
        assert_eq!(connection_symbol(&conn(false, false, false, true)), "╶─");
    }

    #[test]
    fn test_empty_connection() {
        assert_eq!(connection_symbol(&conn(false, false, false, false)), "  ");
    }

    #[test]
    fn test_task_and_empty_cells() {
        assert_eq!(connection_symbol(&Cell::Task("test".to_string())), "  ");
        assert_eq!(connection_symbol(&Cell::Empty), "  ");
    }
}
