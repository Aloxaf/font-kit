// font-kit/src/source.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use descriptor::{FamilySpec, Spec};
use error::SelectionError;
use family::{Family, FamilyHandle};
use font::{Face, Font};
use handle::Handle;
use matching::{self, MatchFields};

#[cfg(all(target_os = "macos", not(feature = "source-fontconfig-default")))]
pub use sources::core_text::CoreTextSource as SystemSource;
#[cfg(all(target_family = "windows", not(feature = "source-fontconfig-default")))]
pub use sources::directwrite::DirectWriteSource as SystemSource;
#[cfg(any(not(any(target_os = "android", target_os = "macos", target_family = "windows")),
          feature = "source-fontconfig-default"))]
pub use sources::fontconfig::FontconfigSource as SystemSource;
#[cfg(all(target_os = "android", not(feature = "source-fontconfig-default")))]
pub use sources::fs::FsSource as SystemSource;

// FIXME(pcwalton): These could expand to multiple fonts, and they could be language-specific.
const DEFAULT_FONT_FAMILY_SERIF: &'static str = "Times New Roman";
const DEFAULT_FONT_FAMILY_SANS_SERIF: &'static str = "Arial";
const DEFAULT_FONT_FAMILY_MONOSPACE: &'static str = "Courier New";
const DEFAULT_FONT_FAMILY_CURSIVE: &'static str = "Comic Sans MS";
const DEFAULT_FONT_FAMILY_FANTASY: &'static str = "Papyrus";

pub trait Source {
    fn all_families(&self) -> Result<Vec<String>, SelectionError>;

    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError>;

    /// The default implementation, which is used by the DirectWrite and the filesystem backends,
    /// does a brute-force search of installed fonts to find the one that matches.
    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<Handle, SelectionError> {
        // TODO(pcwalton): Optimize this by searching for families with similar names first.
        for family_name in try!(self.all_families()) {
            if let Ok(family_handle) = self.select_family_by_name(&family_name) {
                if let Ok(family) = Family::<Font>::from_handle(&family_handle) {
                    for (handle, font) in family_handle.fonts().iter().zip(family.fonts().iter()) {
                        if font.postscript_name() == postscript_name {
                            return Ok((*handle).clone())
                        }
                    }
                }
            }
        }
        Err(SelectionError::NotFound)
    }

    // FIXME(pcwalton): This only returns one family instead of multiple families for the generic
    // family names.
    #[doc(hidden)]
    fn select_family_by_spec(&self, family: &FamilySpec) -> Result<FamilyHandle, SelectionError> {
        match *family {
            FamilySpec::Name(ref name) => self.select_family_by_name(name),
            FamilySpec::Serif => self.select_family_by_name(DEFAULT_FONT_FAMILY_SERIF),
            FamilySpec::SansSerif => self.select_family_by_name(DEFAULT_FONT_FAMILY_SANS_SERIF),
            FamilySpec::Monospace => self.select_family_by_name(DEFAULT_FONT_FAMILY_MONOSPACE),
            FamilySpec::Cursive => self.select_family_by_name(DEFAULT_FONT_FAMILY_CURSIVE),
            FamilySpec::Fantasy => self.select_family_by_name(DEFAULT_FONT_FAMILY_FANTASY),
        }
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and returns the
    /// font handle.
    #[inline]
    fn select_best_match(&self, spec: &Spec) -> Result<Handle, SelectionError> {
        for family in &spec.families {
            if let Ok(family_handle) = self.select_family_by_spec(family) {
                let candidates = try!(self.select_match_fields_for_family(&family_handle));
                if let Ok(index) = matching::find_best_match(&candidates, &spec.properties) {
                    return Ok(family_handle.fonts[index].clone())
                }
            }
        }
        Err(SelectionError::NotFound)
    }

    #[doc(hidden)]
    fn select_match_fields_for_family(&self, family: &FamilyHandle)
                                      -> Result<Vec<MatchFields>, SelectionError> {
        let mut fields = vec![];
        for font_handle in family.fonts() {
            let font = Font::from_handle(font_handle).unwrap();
            let (family_name, properties) = (font.family_name(), font.properties());
            fields.push(MatchFields {
                family_name,
                properties,
            })
        }
        Ok(fields)
    }
}
