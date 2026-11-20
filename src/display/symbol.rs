const DIGIT_WIDTH: usize = 26;
const SEGMENT_THICKNESS: usize = 4;
const DIGIT_GAP: usize = SEGMENT_THICKNESS;
const DIGIT_ADVANCE: usize = DIGIT_GAP + DIGIT_WIDTH - LEFT_COLUMN_X;

const VERTICAL_LENGTH: usize = (super::DISPLAY_HEIGHT - 3 * SEGMENT_THICKNESS) / 2;
const LEFT_COLUMN_X: usize = 2 * SEGMENT_THICKNESS;
const RIGHT_COLUMN_X: usize = DIGIT_WIDTH - SEGMENT_THICKNESS;
const BAR_X: usize = LEFT_COLUMN_X + SEGMENT_THICKNESS;
const BAR_WIDTH: usize = RIGHT_COLUMN_X - BAR_X;
const UPPER_Y: usize = SEGMENT_THICKNESS;
const MIDDLE_Y: usize = UPPER_Y + VERTICAL_LENGTH;
const LOWER_Y: usize = MIDDLE_Y + SEGMENT_THICKNESS;
const BOTTOM_Y: usize = LOWER_Y + VERTICAL_LENGTH;
const COLON_DOT_X: usize = 2 * SEGMENT_THICKNESS + SEGMENT_THICKNESS / 2;
const COLON_ADVANCE: usize = COLON_DOT_X + SEGMENT_THICKNESS + DIGIT_GAP - LEFT_COLUMN_X;

#[derive(Copy, Clone)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Copy, Clone)]
pub enum Segment {
    Top,
    UpperLeft,
    UpperRight,
    Middle,
    LowerLeft,
    LowerRight,
    Bottom,
    ColonTop,
    ColonBottom,
}

use Segment::{
    Bottom, ColonBottom, ColonTop, LowerLeft, LowerRight, Middle, Top, UpperLeft, UpperRight,
};

const fn convert_segment_to_rect(segment: Segment) -> Rect {
    match segment {
        Top => Rect {
            x: BAR_X,
            y: 0,
            width: BAR_WIDTH,
            height: SEGMENT_THICKNESS,
        },
        UpperLeft => Rect {
            x: LEFT_COLUMN_X,
            y: UPPER_Y,
            width: SEGMENT_THICKNESS,
            height: VERTICAL_LENGTH,
        },
        UpperRight => Rect {
            x: RIGHT_COLUMN_X,
            y: UPPER_Y,
            width: SEGMENT_THICKNESS,
            height: VERTICAL_LENGTH,
        },
        Middle => Rect {
            x: BAR_X,
            y: MIDDLE_Y,
            width: BAR_WIDTH,
            height: SEGMENT_THICKNESS,
        },
        LowerLeft => Rect {
            x: LEFT_COLUMN_X,
            y: LOWER_Y,
            width: SEGMENT_THICKNESS,
            height: VERTICAL_LENGTH,
        },
        LowerRight => Rect {
            x: RIGHT_COLUMN_X,
            y: LOWER_Y,
            width: SEGMENT_THICKNESS,
            height: VERTICAL_LENGTH,
        },
        Bottom => Rect {
            x: BAR_X,
            y: BOTTOM_Y,
            width: BAR_WIDTH,
            height: SEGMENT_THICKNESS,
        },
        ColonTop => Rect {
            x: COLON_DOT_X,
            y: MIDDLE_Y - SEGMENT_THICKNESS,
            width: SEGMENT_THICKNESS,
            height: SEGMENT_THICKNESS,
        },
        ColonBottom => Rect {
            x: COLON_DOT_X,
            y: MIDDLE_Y + SEGMENT_THICKNESS,
            width: SEGMENT_THICKNESS,
            height: SEGMENT_THICKNESS,
        },
    }
}

const DIGIT_SEGMENTS: &[&[Segment]] = &[
    &[Top, UpperLeft, UpperRight, LowerLeft, LowerRight, Bottom],
    &[UpperRight, LowerRight],
    &[Top, UpperRight, Middle, LowerLeft, Bottom],
    &[Top, UpperRight, LowerRight, Middle, Bottom],
    &[UpperLeft, UpperRight, Middle, LowerRight],
    &[Top, UpperLeft, Middle, LowerRight, Bottom],
    &[Top, UpperLeft, LowerLeft, Middle, LowerRight, Bottom],
    &[Top, UpperRight, LowerRight],
    &[
        Top, UpperLeft, UpperRight, Middle, LowerLeft, LowerRight, Bottom,
    ],
    &[Top, UpperLeft, UpperRight, Middle, LowerRight, Bottom],
];

const COLON_SEGMENTS: &[Segment] = &[ColonTop, ColonBottom];

pub struct Symbol {
    pub x: usize,
    pub segments: &'static [Segment],
}

impl Symbol {
    pub fn iterate_rectangles(&self) -> impl Iterator<Item = Rect> {
        self.segments.iter().copied().map(convert_segment_to_rect)
    }

    fn create_digit(value: u8, x: usize) -> Self {
        Self {
            x,
            segments: DIGIT_SEGMENTS[(value % 10) as usize],
        }
    }

    fn create_colon(x: usize) -> Self {
        Self {
            x,
            segments: COLON_SEGMENTS,
        }
    }
}

pub fn build_number(value: u32, right_x: usize) -> [Symbol; 5] {
    let clamped = value.min(99999);

    core::array::from_fn(|index| {
        let power = 10u32.pow(4 - index as u32);
        let digit = ((clamped / power) % 10) as u8;
        let x = right_x.saturating_sub(DIGIT_WIDTH + DIGIT_ADVANCE * (4 - index));

        Symbol::create_digit(digit, x)
    })
}

pub fn build_time(total_minutes: u32, start_x: usize) -> [Symbol; 4] {
    let hours = (total_minutes / 60) as u8;
    let minutes = total_minutes % 60;

    let colon_x = start_x + DIGIT_WIDTH + DIGIT_GAP - COLON_DOT_X;
    let colon_right = colon_x + COLON_ADVANCE;

    [
        Symbol::create_digit(hours, start_x),
        Symbol::create_colon(colon_x),
        Symbol::create_digit((minutes / 10) as u8, colon_right),
        Symbol::create_digit((minutes % 10) as u8, colon_right + DIGIT_ADVANCE),
    ]
}
