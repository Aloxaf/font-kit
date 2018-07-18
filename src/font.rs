// font-kit/src/font.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use canvas::{Canvas, RasterizationOptions};
use euclid::{Point2D, Rect, Vector2D};
use lyon_path::builder::PathBuilder;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use descriptor::Properties;
use error::{FontLoadingError, GlyphLoadingError};

pub use loaders::default::Font;

pub trait Face: Clone + Sized {
    type NativeFont;

    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, FontLoadingError>;

    fn from_file(file: &mut File, font_index: u32) -> Result<Self, FontLoadingError>;

    fn from_path<P>(path: P, font_index: u32) -> Result<Self, FontLoadingError>
                    where P: AsRef<Path> {
        Face::from_file(&mut try!(File::open(path)), font_index)
    }

    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self;

    fn from_font<F>(other_font: F) -> Result<Self, FontLoadingError> where F: Face {
        // TODO(pcwalton): This should retrieve the proper font index!
        match other_font.copy_font_data() {
            Some(font_data) => Self::from_bytes(font_data, 0),
            None => Err(FontLoadingError::FontDataUnavailable),
        }
    }

    fn analyze_file(file: &mut File) -> Result<Type, FontLoadingError>;

    #[inline]
    fn analyze_path<P>(path: P) -> Result<Type, FontLoadingError> where P: AsRef<Path> {
        <Self as Face>::analyze_file(&mut try!(File::open(path)))
    }

    /// PostScript name of the font.
    fn postscript_name(&self) -> String;

    /// Full name of the font (also known as "display name" on macOS).
    fn full_name(&self) -> String;

    /// Name of the font family.
    fn family_name(&self) -> String;

    /// Whether the font is monospace (fixed-width).
    fn is_monospace(&self) -> bool;

    /// Various font properties, corresponding to those defined in CSS.
    fn properties(&self) -> Properties;

    fn glyph_for_char(&self, character: char) -> Option<u32>;

    fn outline<B>(&self, glyph_id: u32, hinting_mode: HintingOptions, path_builder: &mut B)
                  -> Result<(), GlyphLoadingError>
                  where B: PathBuilder;

    fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError>;

    fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError>;

    fn origin(&self, glyph_id: u32) -> Result<Point2D<f32>, GlyphLoadingError>;

    fn metrics(&self) -> Metrics;

    fn copy_font_data(&self) -> Option<Arc<Vec<u8>>>;

    fn supports_hinting_options(&self, hinting_options: HintingOptions, for_rasterization: bool)
                                -> bool;

    fn raster_bounds(&self,
                     glyph_id: u32,
                     point_size: f32,
                     origin: &Point2D<f32>,
                     _: HintingOptions,
                     _: RasterizationOptions)
                     -> Result<Rect<i32>, GlyphLoadingError> {
        let typographic_bounds = try!(self.typographic_bounds(glyph_id));
        let typographic_raster_bounds = typographic_bounds * point_size /
            self.metrics().units_per_em as f32;
        Ok(typographic_raster_bounds.translate(&origin.to_vector()).round_out().to_i32())
    }

    fn rasterize_glyph(&self,
                       canvas: &mut Canvas,
                       glyph_id: u32,
                       point_size: f32,
                       origin: &Point2D<f32>,
                       hinting_options: HintingOptions,
                       rasterization_options: RasterizationOptions)
                       -> Result<(), GlyphLoadingError>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    Single,
    Collection(u32),
}

/// Various metrics that apply to the entire font.
///
/// For OpenType fonts, these mostly come from the `OS/2` table.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,

    /// The maximum amount the font rises above the baseline, in font units.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in font units.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed.
    pub descent: f32,

    /// Distance between baselines, in font units.
    pub line_gap: f32,

    pub underline_position: f32,

    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in font units.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HintingOptions {
    /// No hinting is performed unless absolutely necessary to assemble the glyph.
    ///
    /// This corresponds to what macOS and FreeType in its "no hinting" mode do.
    None,

    /// Hinting is performed only in the vertical direction. The specified point size is used for
    /// grid fitting.
    ///
    /// This corresponds to what DirectWrite and FreeType in its light hinting mode do.
    Vertical(f32),

    /// Hinting is performed only in the vertical direction, and further tweaks are applied to make
    /// subpixel antialiasing look better. The specified point size is used for grid fitting.
    ///
    /// This matches DirectWrite, GDI in its ClearType mode, and FreeType in its LCD hinting mode.
    VerticalSubpixel(f32),

    /// Hinting is performed in both horizontal and vertical directions. The specified point size
    /// is used for grid fitting.
    ///
    /// This corresponds to what GDI in non-ClearType modes and FreeType in its normal hinting mode
    /// do.
    Full(f32),
}

impl HintingOptions {
    #[inline]
    pub fn grid_fitting_size(&self) -> Option<f32> {
        match *self {
            HintingOptions::None => None,
            HintingOptions::Vertical(size) |
            HintingOptions::VerticalSubpixel(size) |
            HintingOptions::Full(size) => {
                Some(size)
            }
        }
    }
}
