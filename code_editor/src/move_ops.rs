use crate::{selection::Affinity, str::StrExt, Point, Session};

pub fn move_left(lines: &[String], point: Point) -> Point {
    if !is_at_start_of_line(point) {
        return move_to_prev_grapheme(lines, point);
    }
    if !is_at_first_line(point) {
        return move_to_end_of_prev_line(lines, point);
    }
    point
}

pub fn move_right(lines: &[String], point: Point) -> Point {
    if !is_at_end_of_line(lines, point) {
        return move_to_next_grapheme(lines, point);
    }
    if !is_at_last_line(lines, point) {
        return move_to_start_of_next_line(point);
    }
    point
}

pub fn move_up(
    session: &Session,
    point: Point,
    affinity: Affinity,
    preferred_column: Option<usize>,
) -> (Point, Affinity, Option<usize>) {
    if !is_at_first_row_of_line(session, point, affinity) {
        return move_to_prev_row_of_line(session, point, affinity, preferred_column);
    }
    if !is_at_first_line(point) {
        return move_to_last_row_of_prev_line(session, point, affinity, preferred_column);
    }
    (point, affinity, preferred_column)
}

pub fn move_down(
    session: &Session,
    point: Point,
    affinity: Affinity,
    preferred_column: Option<usize>,
) -> (Point, Affinity, Option<usize>) {
    if !is_at_last_row_of_line(session, point, affinity) {
        return move_to_next_row_of_line(session, point, affinity, preferred_column);
    }
    if !is_at_last_line(session.document().borrow().text().as_lines(), point) {
        return move_to_first_row_of_next_line(session, point, affinity, preferred_column);
    }
    (point, affinity, preferred_column)
}

fn is_at_first_line(point: Point) -> bool {
    point.line == 0
}

fn is_at_last_line(lines: &[String], point: Point) -> bool {
    point.line == lines.len()
}

fn is_at_start_of_line(point: Point) -> bool {
    point.byte == 0
}

fn is_at_end_of_line(lines: &[String], point: Point) -> bool {
    point.byte == lines[point.line].len()
}

fn is_at_first_row_of_line(session: &Session, point: Point, affinity: Affinity) -> bool {
    session.line(point.line, |line| {
        let (row, _) = line.byte_and_affinity_to_row_and_column(
            point.byte,
            affinity,
            session.settings().tab_column_count,
        );
        row == 0
    })
}

fn is_at_last_row_of_line(session: &Session, point: Point, affinity: Affinity) -> bool {
    session.line(point.line, |line| {
        let (row, _) = line.byte_and_affinity_to_row_and_column(
            point.byte,
            affinity,
            session.settings().tab_column_count,
        );
        row == line.row_count() - 1
    })
}

fn move_to_prev_grapheme(lines: &[String], point: Point) -> Point {
    Point {
        line: point.line,
        byte: lines[point.line][..point.byte]
            .grapheme_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap(),
    }
}

fn move_to_next_grapheme(lines: &[String], point: Point) -> Point {
    let line = &lines[point.line];
    Point {
        line: point.line,
        byte: line[point.byte..]
            .grapheme_indices()
            .nth(1)
            .map(|(index, _)| point.byte + index)
            .unwrap_or(line.len()),
    }
}

fn move_to_end_of_prev_line(lines: &[String], point: Point) -> Point {
    let prev_line = point.line - 1;
    Point {
        line: prev_line,
        byte: lines[prev_line].len(),
    }
}

fn move_to_start_of_next_line(point: Point) -> Point {
    Point {
        line: point.line + 1,
        byte: 0,
    }
}

fn move_to_prev_row_of_line(
    session: &Session,
    point: Point,
    affinity: Affinity,
    preferred_column: Option<usize>,
) -> (Point, Affinity, Option<usize>) {
    session.line(point.line, |line| {
        let (row, mut column) = line.byte_and_affinity_to_row_and_column(
            point.byte,
            affinity,
            session.settings().tab_column_count,
        );
        if let Some(preferred_column) = preferred_column {
            column = preferred_column;
        }
        let (byte, affinity) = line.row_and_column_to_byte_and_affinity(
            row - 1,
            column,
            session.settings().tab_column_count,
        );
        (
            Point {
                line: point.line,
                byte,
            },
            affinity,
            Some(column),
        )
    })
}

fn move_to_next_row_of_line(
    session: &Session,
    point: Point,
    affinity: Affinity,
    preferred_column: Option<usize>,
) -> (Point, Affinity, Option<usize>) {
    session.line(point.line, |line| {
        let (row, mut column) = line.byte_and_affinity_to_row_and_column(
            point.byte,
            affinity,
            session.settings().tab_column_count,
        );
        if let Some(preferred_column) = preferred_column {
            column = preferred_column;
        }
        let (byte, affinity) = line.row_and_column_to_byte_and_affinity(
            row + 1,
            column,
            session.settings().tab_column_count,
        );
        (
            Point {
                line: point.line,
                byte,
            },
            affinity,
            Some(column),
        )
    })
}

fn move_to_last_row_of_prev_line(
    session: &Session,
    point: Point,
    affinity: Affinity,
    preferred_column: Option<usize>,
) -> (Point, Affinity, Option<usize>) {
    session.line(point.line, |line| {
        let (_, mut column) = line.byte_and_affinity_to_row_and_column(
            point.byte,
            affinity,
            session.settings().tab_column_count,
        );
        if let Some(preferred_column) = preferred_column {
            column = preferred_column;
        }
        session.line(point.line - 1, |prev_line| {
            let (byte, affinity) = prev_line.row_and_column_to_byte_and_affinity(
                prev_line.row_count() - 1,
                column,
                session.settings().tab_column_count,
            );
            (
                Point {
                    line: point.line - 1,
                    byte,
                },
                affinity,
                Some(column),
            )
        })
    })
}

fn move_to_first_row_of_next_line(
    session: &Session,
    point: Point,
    affinity: Affinity,
    preferred_column: Option<usize>,
) -> (Point, Affinity, Option<usize>) {
    session.line(point.line, |line| {
        let (_, mut column) = line.byte_and_affinity_to_row_and_column(
            point.byte,
            affinity,
            session.settings().tab_column_count,
        );
        if let Some(preferred_column) = preferred_column {
            column = preferred_column;
        }
        session.line(point.line + 1, |next_line| {
            let (byte, affinity) = next_line.row_and_column_to_byte_and_affinity(
                0,
                column,
                session.settings().tab_column_count,
            );
            (
                Point {
                    line: point.line + 1,
                    byte,
                },
                affinity,
                Some(column),
            )
        })
    })
}
