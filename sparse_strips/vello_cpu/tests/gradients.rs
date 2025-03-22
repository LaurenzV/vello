mod util;

use crate::util::{check_ref, get_ctx, render_pixmap, star_path};
use vello_common::color::palette::css::{BLACK, BLUE, GREEN, RED, WHITE, YELLOW};
use vello_common::kurbo::{Affine, Point, Rect};
use vello_common::paint::{LinearGradient, Stop};

#[test]
fn gradient_on_3_wide_tiles() {
    let mut ctx = get_ctx(600, 32, false);
    let rect = Rect::new(4.0, 4.0, 596.0, 28.0);

    let gradient = LinearGradient {
        p0: Point::new(0.0, 0.0),
        p1: Point::new(600.0, 0.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_on_3_wide_tiles");
}

#[test]
fn gradient_linear_2_stops() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(20.0, 0.0),
        p1: Point::new(180.0, 0.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_2_stops");
}

#[test]
fn gradient_linear_2_stops_with_alpha() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(20.0, 0.0),
        p1: Point::new(180.0, 0.0),
        stops: stops_green_blue_with_alpha(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_linear_2_stops_with_alpha");
}

#[test]
fn gradient_linear_negative_direction() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(180.0, 0.0),
        p1: Point::new(20.0, 0.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_linear_negative_direction");
}

#[test]
fn gradient_spread_method_pad() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(70.0, 0.0),
        p1: Point::new(130.0, 0.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_spread_method_pad");
}

#[test]
fn gradient_spread_method_repeat() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(90.0, 0.0),
        p1: Point::new(110.0, 0.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Repeat,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_spread_method_repeat");
}

#[test]
fn gradient_spread_method_reflect() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(90.0, 0.0),
        p1: Point::new(110.0, 0.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Reflect,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_spread_method_reflect");
}

#[test]
fn gradient_linear_4_stops() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(20.0, 0.0),
        p1: Point::new(180.0, 0.0),
        stops: stops_blue_green_red_yellow(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_4_stops");
}

#[test]
fn gradient_complex_shape() {
    let mut ctx = get_ctx(200, 200, false);
    let path = Affine::scale(2.0) * star_path();

    let gradient = LinearGradient {
        p0: Point::new(0.0, 0.0),
        p1: Point::new(200.0, 0.0),
        stops: stops_blue_green_red_yellow(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_path(&path);

    check_ref(&ctx, "gradient_complex_shape");
}

#[test]
fn gradient_linear_with_y_pad() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(40.0, 40.0),
        p1: Point::new(160.0, 160.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Pad,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_linear_with_y_pad");
}

#[test]
fn gradient_linear_with_y_repeat() {
    let mut ctx = get_ctx(200, 200, false);
    let rect = Rect::new(20.0, 20.0, 180.0, 180.0);

    let gradient = LinearGradient {
        p0: Point::new(95.0, 95.0),
        p1: Point::new(101.0, 105.0),
        stops: stops_green_blue(),
        extend: vello_common::peniko::Extend::Repeat,
    };

    ctx.set_paint(gradient.into());
    ctx.fill_rect(&rect);

    check_ref(&ctx, "gradient_linear_with_y_repeat");
}

fn stops_green_blue() -> Vec<Stop> {
    vec![
        Stop {
            offset: 0.0,
            color: GREEN,
        },
        Stop {
            offset: 1.0,
            color: BLUE,
        },
    ]
}

fn stops_green_blue_with_alpha() -> Vec<Stop> {
    vec![
        Stop {
            offset: 0.0,
            color: GREEN.with_alpha(0.25),
        },
        Stop {
            offset: 1.0,
            color: BLUE.with_alpha(0.75),
        },
    ]
}

fn stops_black_white() -> Vec<Stop> {
    vec![
        Stop {
            offset: 0.0,
            color: BLACK,
        },
        Stop {
            offset: 1.0,
            color: WHITE,
        },
    ]
}

fn stops_blue_green_red_yellow() -> Vec<Stop> {
    vec![
        Stop {
            offset: 0.0,
            color: BLUE,
        },
        Stop {
            offset: 0.33,
            color: GREEN,
        },
        Stop {
            offset: 0.66,
            color: RED,
        },
        Stop {
            offset: 1.0,
            color: YELLOW,
        },
    ]
}
