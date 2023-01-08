// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! `gettext` for `mdbook`
//!
//! This program works like `gettext`, meaning it will translate
//! strings in your book.
//!
//! The translations come from GNU Gettext `xx.po` files. You must set
//! preprocessor.gettext.po-file to the PO file to use. If unset, a
//! warning is issued while building the book.
//!
//! See `TRANSLATIONS.md` in the repository root for more information.

use anyhow::{anyhow, Context};
use i18n_helpers::extract_paragraphs;
use mdbook::book::Book;
use mdbook::preprocess::{CmdPreprocessor, PreprocessorContext};
use mdbook::BookItem;
use polib::catalog::Catalog;
use polib::po_file;
use semver::{Version, VersionReq};
use std::io;
use std::path::Path;
use std::process;

fn translate(text: &str, catalog: &Catalog) -> String {
    let mut output = String::with_capacity(text.len());
    let mut current_lineno = 1;

    for (lineno, paragraph) in extract_paragraphs(text) {
        // Fill in blank lines between paragraphs. This is important
        // for code blocks where blank lines are significant.
        while current_lineno < lineno {
            output.push('\n');
            current_lineno += 1;
        }
        current_lineno += paragraph.lines().count();

        let translated = catalog
            .find_message(paragraph)
            .and_then(|msg| msg.get_msgstr().ok())
            .filter(|msgstr| !msgstr.is_empty())
            .map(|msgstr| msgstr.as_str())
            .unwrap_or(paragraph);
        output.push_str(translated);
    }

    if text.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn translate_book(ctx: &PreprocessorContext, mut book: Book) -> anyhow::Result<Book> {
    let cfg = ctx
        .config
        .get_preprocessor("gettext")
        .ok_or_else(|| anyhow!("Could not read preprocessor.gettext configuration"))?;
    let po_file = cfg
        .get("po-file")
        .ok_or_else(|| anyhow!("Missing preprocessor.gettext.po-file config value"))?;
    let path = po_file.as_str().ok_or_else(|| {
        anyhow!(
            "Expected a string for preprocessor.gettext.po-file, found {po_file} ({})",
            po_file.type_str()
        )
    })?;
    let catalog = po_file::parse(Path::new(path))
        .map_err(|err| anyhow!("{err}"))
        .with_context(|| format!("Could not parse {path} as PO file"))?;

    book.for_each_mut(|item| match item {
        BookItem::Chapter(ch) => {
            ch.content = translate(&ch.content, &catalog);
            ch.name = translate(&ch.name, &catalog);
        }
        BookItem::Separator => {}
        BookItem::PartTitle(title) => {
            *title = translate(title, &catalog);
        }
    });

    Ok(book)
}

fn preprocess() -> anyhow::Result<()> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;
    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;
    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: The gettext preprocessor was built against \
             mdbook version {}, but we're being called from version {}",
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let translated_book = translate_book(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &translated_book)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    if std::env::args().len() == 3 {
        assert_eq!(std::env::args().nth(1).as_deref(), Some("supports"));
        // Signal that we support all renderers.
        process::exit(0);
    }

    preprocess()
}
