use std::env;

use ansi_term::Colour::Fixed;
use image::{GenericImageView, Pixel, Rgb};

type Px = Rgb<u8>;

fn main() {
    let args: Vec<String> = env::args().collect();

    let (term_w, term_h) = term_size::dimensions().expect("Unable to get terminal size");

    let image = image::io::Reader::open(args.get(1).expect("Usage: tty-image [FILE]"))
        .expect("Failed to open image file")
        .decode()
        .expect("Failed to parse image file");

    for y in 0..(image.height() / 4).min(term_h as u32) {
        for x in 0..(image.width() / 2).min(term_w as u32) {
            let view = image.view(x * 2, y * 4, 2, 4);

            // Stupid quantization, just pick the darkest and brightest
            // pixels.

            let mut min: Px = view.pixels().next().unwrap().2.to_rgb();
            let mut max: Px = min;

            for (_, _, p) in view.pixels() {
                let p = p.to_rgb();
                if distance2(p, Rgb([0, 0, 0])) < distance2(min, Rgb([0, 0, 0])) {
                    min = p;
                }
                if distance2(p, Rgb([0xff, 0xff, 0xff])) < distance2(max, Rgb([0xff, 0xff, 0xff])) {
                    max = p;
                }
            }

            let b = to_xterm256(min);
            let f = to_xterm256(max);

            let mut mask = 0;
            // Offsets of pseudopixels in Unicode braille chars.
            for (i, &(x, y)) in [
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 1),
                (1, 2),
                (0, 3),
                (1, 3),
            ]
            .iter()
            .enumerate()
            {
                let p = view.get_pixel(x, y).to_rgb();
                if distance2(p, max) < distance2(p, min) {
                    mask |= 1 << i;
                }
            }
            // TODO: Using weird enumeration order of the braille chars,
            // create a bitmask of the pixels that are closer to max than min.

            print!(
                "{}",
                Fixed(f)
                    .on(Fixed(b))
                    .paint(format!("{}", char::from_u32(0x2800 + mask).unwrap()))
            );
        }
        println!();
    }
}

/// Return square of distance to other color
fn distance2(one: Px, other: Px) -> i32 {
    let x = one[0] as i32 - other[0] as i32;
    let y = one[1] as i32 - other[1] as i32;
    let z = one[2] as i32 - other[2] as i32;

    x * x + y * y + z * z
}

// Convert 24-bit color into 256 color terminal color.
fn to_xterm256(c: Px) -> u8 {
    // Never convert into the first 16 colors, the user may have configured those to have
    // different RGB values.

    fn rgb_channel(c: u8) -> u8 {
        if c < 48 {
            0
        } else if c < 75 {
            1
        } else {
            (c - 35) / 40
        }
    }

    fn gray_channel(c: u8) -> u8 {
        if c < 13 {
            0
        } else if c > 235 {
            23
        } else {
            (c - 3) / 10
        }
    }

    let rgb_color = {
        let r = rgb_channel(c[0]);
        let g = rgb_channel(c[1]);
        let b = rgb_channel(c[2]);

        16 + b + g * 6 + r * 36
    };

    let gray_color = {
        // This doesn't do any of the weighting tricks you're supposed to do when converting
        // chromatic color into grayscale.
        // It should be okay though since this should only end up applied to colors which are
        // naturally very close to gray to begin with, since the other colors get snapped by
        // the more chromatic palette colors.
        let gray = ((c[0] as u32 + c[1] as u32 + c[2] as u32) / 3) as u8;
        232 + gray_channel(gray)
    };

    if distance2(c, from_xterm256(rgb_color)) < distance2(c, from_xterm256(gray_color)) {
        rgb_color
    } else {
        gray_color
    }
}

const fn from_xterm256(c: u8) -> Px {
    // The first 16 are the EGA colors
    if c == 7 {
        Rgb([192, 192, 192])
    } else if c == 8 {
        Rgb([128, 128, 128])
    } else if c < 16 {
        let i = if c & 0b1000 != 0 { 255 } else { 128 };
        let r = if c & 0b1 != 0 { i } else { 0 };
        let g = if c & 0b10 != 0 { i } else { 0 };
        let b = if c & 0b100 != 0 { i } else { 0 };

        Rgb([r * i, g * i, b * i])
    } else if c < 232 {
        const fn channel(i: u8) -> u8 {
            i * 40 + if i > 0 { 55 } else { 0 }
        }
        // 6^3 RGB space
        let c = c - 16;
        let b = channel(c % 6);
        let g = channel((c / 6) % 6);
        let r = channel(c / 36);

        Rgb([r, g, b])
    } else {
        // 24 level grayscale slide
        let c = c - 232;
        let c = 8 + 10 * c;
        Rgb([c, c, c])
    }
}
