use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use word_core::{
    collect_ordered_note_references, sanitize_table_cell_background_color,
    sanitize_table_column_widths, validate_note_reference, AssetRef, Block, Document, Heading,
    ImageAlignment, ImageBlock, Inline, InlineMark, NoteKind, PageField, PageRegion,
    PageRegionBlock, PageSetup, Paragraph, ParagraphAlignment, ParagraphFormat, Section, Style,
    StyleKind, TableCellBorder, TableCellPresentation, TableOfContents,
};

const POINTS_PER_MM: f32 = 72.0 / 25.4;
const PDF_FONT_SIZE: f32 = 11.0;
const PDF_REGION_FONT_SIZE: f32 = 9.0;
const PDF_LEADING: f32 = 14.0;
const PDF_REGION_LEADING: f32 = 12.0;
const PDF_MIN_MARGIN_POINTS: f32 = 24.0;
const PDF_REGION_GAP_POINTS: f32 = 10.0;
const PDF_TABLE_FONT_SIZE: f32 = 9.5;
const PDF_TABLE_LEADING: f32 = 11.5;
const PDF_TABLE_CELL_PADDING: f32 = 3.5;
const PDF_TABLE_ROW_GAP_POINTS: f32 = 3.0;
const PDF_FIGURE_FONT_SIZE: f32 = 9.5;
const PDF_FIGURE_LEADING: f32 = 11.5;
const PDF_FIGURE_PADDING: f32 = 6.0;
const PDF_FIGURE_BASE_HEIGHT: f32 = 72.0;
const PDF_FIGURE_MIN_HEIGHT: f32 = 48.0;
const PDF_FIGURE_GAP_POINTS: f32 = 4.0;
const PDF_FIGURE_IMAGE_TEXT_GAP_POINTS: f32 = 4.0;
const PDF_TEXT_WIDTH_FACTOR: f32 = 0.52;
const PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT: usize = 512;
const PDF_MAX_LINK_ANNOTATIONS_PER_PAGE: usize = 64;
const PDF_MAX_URI_BYTES: usize = 2048;
const PDF_MAX_EMBEDDED_IMAGES_PER_DOCUMENT: usize = 32;
const PDF_MAX_JPEG_BYTES_PER_IMAGE: usize = 8 * 1024 * 1024;
const PDF_MAX_JPEG_DIMENSION: u32 = 8192;
const PDF_MAX_JPEG_PIXELS: u64 = 20_000_000;
const PDF_PAGE_NUMBER_TOKEN: &str = "\u{e000}page-number\u{e001}";
const PDF_PAGE_COUNT_TOKEN: &str = "\u{e000}page-count\u{e001}";
const PDF_DATE_TOKEN: &str = "\u{e000}date\u{e001}";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExportError {
    #[error("document has no sections")]
    EmptyDocument,
    #[error("PDF page range is invalid")]
    InvalidPdfPageRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PdfExportOptions {
    pub page_range: Option<PdfPageRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfPageRange {
    pub start: usize,
    pub end: usize,
}

pub fn export_txt(document: &Document) -> Result<String, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let mut output = String::new();
    for section in &document.sections {
        push_section_regions_text(section, document, true, &mut output);
        for block in &section.blocks {
            push_block_text(block, document, &mut output);
            output.push('\n');
        }
        push_section_regions_text(section, document, false, &mut output);
    }
    push_notes_text(document, &mut output);
    Ok(output.trim_end().to_string())
}

pub fn export_html(document: &Document) -> Result<String, ExportError> {
    export_html_with_options(document, HtmlExportOptions::default())
}

pub fn export_print_html(document: &Document) -> Result<String, ExportError> {
    export_html_with_options(document, HtmlExportOptions { print_ready: true })
}

pub fn export_basic_pdf(document: &Document) -> Result<Vec<u8>, ExportError> {
    export_pdf_with_options(document, PdfExportOptions::default())
}

pub fn export_pdf_with_options(
    document: &Document,
    options: PdfExportOptions,
) -> Result<Vec<u8>, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let pages = paginate_pdf(document)?;
    let selected_pages = select_pdf_pages(&pages, options.page_range)?;
    Ok(build_pdf(document, selected_pages, pages.len()))
}

#[derive(Debug, Clone)]
struct PdfProjectedText {
    section_index: usize,
    text: PdfLinkedText,
}

struct PdfProjectionState<'a> {
    bookmark_id_counts: &'a BTreeMap<String, usize>,
    active_note_ids: &'a BTreeSet<String>,
    emitted_note_reference_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
struct PdfLinkedText {
    runs: Vec<PdfLinkedRun>,
    pending_destinations: Vec<PdfDestinationId>,
}

#[derive(Debug, Clone)]
struct PdfLinkedRun {
    text: String,
    target: Option<PdfLinkTarget>,
    destinations: Vec<PdfDestinationId>,
}

#[derive(Debug, Clone)]
struct PdfLinkedLine {
    text: String,
    links: Vec<PdfLineLink>,
    destinations: Vec<PdfLineDestination>,
}

#[derive(Debug, Clone)]
struct PdfLineLink {
    start: usize,
    end: usize,
    target: PdfLinkTarget,
}

#[derive(Debug, Clone)]
struct PdfLineDestination {
    offset: usize,
    id: PdfDestinationId,
}

#[derive(Debug, Clone)]
struct PdfLinkedChar {
    ch: char,
    target: Option<PdfLinkTarget>,
    destinations: Vec<PdfDestinationId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PdfLinkTarget {
    Uri(String),
    Destination(PdfDestinationId),
}

#[derive(Debug, Clone)]
struct PdfLinkAnnotation {
    rect: PdfRect,
    target: PdfAnnotationTarget,
}

#[derive(Debug, Clone)]
enum PdfAnnotationTarget {
    Uri(String),
    Destination(PdfDestinationId),
}

#[derive(Debug, Clone)]
struct PdfDestination {
    page_number: usize,
    left: f32,
    top: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PdfDestinationId {
    Bookmark(String),
    NoteBody(String),
    NoteReference(String),
}

#[derive(Debug, Clone, Copy)]
struct PdfRect {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

#[derive(Debug, Clone)]
struct PdfRenderedPage {
    stream: String,
    annotations: Vec<PdfLinkAnnotation>,
    images: Vec<PdfRenderedImage>,
}

struct PdfAnnotationCollector<'a> {
    annotations: &'a mut Vec<PdfLinkAnnotation>,
    remaining: &'a mut usize,
    destinations: &'a BTreeMap<PdfDestinationId, PdfDestination>,
}

struct PdfImageCollector<'a> {
    document: &'a Document,
    images: &'a mut Vec<PdfRenderedImage>,
    remaining: &'a mut usize,
}

struct PdfPageRenderState<'a> {
    cursor_y: &'a mut f32,
    annotations: &'a mut Vec<PdfLinkAnnotation>,
    remaining_annotations: &'a mut usize,
    images: &'a mut Vec<PdfRenderedImage>,
    remaining_images: &'a mut usize,
}

#[derive(Debug, Clone)]
struct PdfRenderedImage {
    name: String,
    bytes: Vec<u8>,
    width_px: u32,
    height_px: u32,
    components: u8,
}

impl PdfLinkedText {
    fn from_plain(text: impl AsRef<str>) -> Self {
        let mut linked = Self::default();
        linked.push_plain(text.as_ref());
        linked
    }

    fn push_plain(&mut self, text: &str) {
        self.push_run(text, None);
    }

    fn push_uri(&mut self, text: &str, uri: &str) {
        self.push_link(text, PdfLinkTarget::Uri(uri.to_string()));
    }

    fn push_destination_link(&mut self, text: &str, destination_id: PdfDestinationId) {
        self.push_link(text, PdfLinkTarget::Destination(destination_id));
    }

    fn push_link(&mut self, text: &str, target: PdfLinkTarget) {
        self.push_run(text, Some(target));
    }

    fn push_destination_marker(&mut self, destination_id: PdfDestinationId) {
        self.pending_destinations.push(destination_id);
    }

    fn is_empty(&self) -> bool {
        self.runs.is_empty() && self.pending_destinations.is_empty()
    }

    fn append(&mut self, other: &PdfLinkedText) {
        for run in &other.runs {
            self.push_run_with_destinations(
                &run.text,
                run.target.clone(),
                run.destinations.clone(),
            );
        }
        self.pending_destinations
            .extend(other.pending_destinations.clone());
    }

    fn take_pending_destinations(&mut self) -> Vec<PdfDestinationId> {
        std::mem::take(&mut self.pending_destinations)
    }

    fn push_run_with_destinations(
        &mut self,
        text: &str,
        target: Option<PdfLinkTarget>,
        mut destinations: Vec<PdfDestinationId>,
    ) {
        if text.is_empty() {
            self.pending_destinations.append(&mut destinations);
            return;
        }
        let mut pending = self.take_pending_destinations();
        pending.append(&mut destinations);
        if let Some(last) = self
            .runs
            .last_mut()
            .filter(|run| run.target == target && pending.is_empty())
        {
            last.text.push_str(text);
            return;
        }
        self.runs.push(PdfLinkedRun {
            text: text.to_string(),
            target,
            destinations: pending,
        });
    }

    fn split_lines(&self) -> Vec<PdfLinkedText> {
        let mut lines = Vec::new();
        let mut current = Vec::new();
        let mut saw_any = false;
        let mut last_was_newline = false;

        for linked_char in self.to_chars() {
            saw_any = true;
            if linked_char.ch == '\n' {
                lines.push(PdfLinkedText::from_chars(current));
                current = Vec::new();
                last_was_newline = true;
            } else {
                current.push(linked_char);
                last_was_newline = false;
            }
        }

        if saw_any && (!current.is_empty() || !last_was_newline) {
            lines.push(PdfLinkedText::from_chars(current));
        }
        if !saw_any && !self.pending_destinations.is_empty() {
            let line = PdfLinkedText {
                pending_destinations: self.pending_destinations.clone(),
                ..Default::default()
            };
            lines.push(line);
        }

        lines
    }

    fn trimmed(&self) -> PdfLinkedText {
        let chars = self.to_chars();
        let Some(start) = chars
            .iter()
            .position(|linked_char| !linked_char.ch.is_whitespace())
        else {
            let mut linked = PdfLinkedText::default();
            for linked_char in chars {
                linked.pending_destinations.extend(linked_char.destinations);
            }
            linked
                .pending_destinations
                .extend(self.pending_destinations.clone());
            return linked;
        };
        let end = chars
            .iter()
            .rposition(|linked_char| !linked_char.ch.is_whitespace())
            .map(|index| index + 1)
            .unwrap_or(start);
        let mut leading_destinations = Vec::new();
        for linked_char in &chars[..start] {
            leading_destinations.extend(linked_char.destinations.clone());
        }
        let mut trimmed = chars[start..end].to_vec();
        if let Some(first) = trimmed.first_mut() {
            let mut destinations = leading_destinations;
            destinations.append(&mut first.destinations);
            first.destinations = destinations;
        }
        let mut linked = PdfLinkedText::from_chars(trimmed);
        linked
            .pending_destinations
            .extend(self.pending_destinations.clone());
        linked
    }

    fn push_run(&mut self, text: &str, target: Option<PdfLinkTarget>) {
        if text.is_empty() {
            return;
        }
        let pending = self.take_pending_destinations();
        if let Some(last) = self
            .runs
            .last_mut()
            .filter(|run| run.target == target && pending.is_empty())
        {
            last.text.push_str(text);
            return;
        }
        self.runs.push(PdfLinkedRun {
            text: text.to_string(),
            target,
            destinations: pending,
        });
    }

    fn to_chars(&self) -> Vec<PdfLinkedChar> {
        let mut chars = Vec::new();
        for run in &self.runs {
            for (index, ch) in run.text.chars().enumerate() {
                chars.push(PdfLinkedChar {
                    ch,
                    target: run.target.clone(),
                    destinations: if index == 0 {
                        run.destinations.clone()
                    } else {
                        Vec::new()
                    },
                });
            }
        }
        chars
    }

    fn from_chars(chars: Vec<PdfLinkedChar>) -> PdfLinkedText {
        let mut linked = PdfLinkedText::default();
        for linked_char in chars {
            let mut text = String::new();
            text.push(linked_char.ch);
            linked.push_run_with_destinations(&text, linked_char.target, linked_char.destinations);
        }
        linked
    }
}

#[derive(Debug, Clone)]
struct PdfProjectedTableRow {
    section_index: usize,
    column_widths: Vec<u16>,
    cells: Vec<PdfProjectedTableCell>,
}

#[derive(Debug, Clone)]
struct PdfProjectedTableCell {
    text: PdfLinkedText,
    presentation: TableCellPresentation,
}

#[derive(Debug, Clone)]
struct PdfProjectedFigure {
    section_index: usize,
    alt_text: Option<String>,
    caption: Option<String>,
    alignment: ImageAlignment,
    scale_percent: u16,
    jpeg: Option<PdfJpegImageCandidate>,
}

#[derive(Debug, Clone)]
struct PdfJpegImageCandidate {
    asset_id: String,
    width_px: u32,
    height_px: u32,
    components: u8,
}

#[derive(Debug, Clone)]
enum PdfFlowItem {
    Text(PdfProjectedText),
    TableRow(PdfProjectedTableRow),
    Figure(PdfProjectedFigure),
    PageBreak { section_index: usize },
}

#[derive(Debug, Clone)]
enum PdfPageBodyItem {
    TextLine(PdfLinkedLine),
    TableRow(PdfTableRowLayout),
    Figure(PdfFigureLayout),
}

impl PdfPageBodyItem {
    fn height(&self) -> f32 {
        match self {
            PdfPageBodyItem::TextLine(_) => PDF_LEADING,
            PdfPageBodyItem::TableRow(row) => row.block_height,
            PdfPageBodyItem::Figure(figure) => figure.block_height,
        }
    }
}

#[derive(Debug, Clone)]
struct PdfTableRowLayout {
    cells: Vec<PdfTableCellLayout>,
    cell_widths: Vec<f32>,
    row_height: f32,
    block_height: f32,
}

#[derive(Debug, Clone)]
struct PdfTableCellLayout {
    lines: Vec<PdfLinkedLine>,
    presentation: TableCellPresentation,
}

#[derive(Debug, Clone)]
struct PdfFigureLayout {
    lines: Vec<String>,
    alignment: ImageAlignment,
    width: f32,
    box_height: f32,
    block_height: f32,
    image: Option<PdfFigureImageLayout>,
}

#[derive(Debug, Clone)]
struct PdfFigureImageLayout {
    asset_id: String,
    width_px: u32,
    height_px: u32,
    components: u8,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Copy)]
struct PdfPageRenderContext<'a> {
    document: &'a Document,
    page: &'a PdfPage,
    total_pages: usize,
    destinations: &'a BTreeMap<PdfDestinationId, PdfDestination>,
    margin_left: f32,
    content_width: f32,
}

struct PdfDestinationCollectionContext<'a> {
    document: &'a Document,
    page: &'a PdfPage,
    total_pages: usize,
    margin_left: f32,
    content_width: f32,
}

#[derive(Debug, Clone)]
struct PdfPage {
    page_number: usize,
    section_index: usize,
    section_page_number: usize,
    page_setup: PageSetup,
    body_items: Vec<PdfPageBodyItem>,
}

fn paginate_pdf(document: &Document) -> Result<Vec<PdfPage>, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let flow = pdf_flow_items(document)?;
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut current_section_index = 0;
    let mut current_body_items = Vec::new();
    let mut current_body_height = 0.0;

    for item in flow {
        match item {
            PdfFlowItem::Text(text) => {
                flush_pdf_page_if_section_changes(
                    document,
                    &mut pages,
                    &mut current_section_index,
                    &mut current_body_items,
                    &mut current_body_height,
                    text.section_index,
                );
                let page_setup = &document.sections[text.section_index].page;
                let max_chars = pdf_wrap_char_limit(page_setup);
                for wrapped in wrap_pdf_linked_line(&text.text, max_chars) {
                    push_pdf_body_item(
                        document,
                        &mut pages,
                        current_section_index,
                        &mut current_body_items,
                        &mut current_body_height,
                        PdfPageBodyItem::TextLine(wrapped),
                    );
                }
            }
            PdfFlowItem::TableRow(row) => {
                flush_pdf_page_if_section_changes(
                    document,
                    &mut pages,
                    &mut current_section_index,
                    &mut current_body_items,
                    &mut current_body_height,
                    row.section_index,
                );
                let content_width = pdf_content_width(&document.sections[row.section_index].page);
                let layout = layout_pdf_table_row(&row, content_width);
                push_pdf_body_item(
                    document,
                    &mut pages,
                    current_section_index,
                    &mut current_body_items,
                    &mut current_body_height,
                    PdfPageBodyItem::TableRow(layout),
                );
            }
            PdfFlowItem::Figure(figure) => {
                flush_pdf_page_if_section_changes(
                    document,
                    &mut pages,
                    &mut current_section_index,
                    &mut current_body_items,
                    &mut current_body_height,
                    figure.section_index,
                );
                let content_width =
                    pdf_content_width(&document.sections[figure.section_index].page);
                let layout = layout_pdf_figure(&figure, content_width);
                push_pdf_body_item(
                    document,
                    &mut pages,
                    current_section_index,
                    &mut current_body_items,
                    &mut current_body_height,
                    PdfPageBodyItem::Figure(layout),
                );
            }
            PdfFlowItem::PageBreak { section_index } => {
                current_section_index = section_index;
                push_pdf_page(
                    document,
                    &mut pages,
                    current_section_index,
                    std::mem::take(&mut current_body_items),
                );
                current_body_height = 0.0;
            }
        }
    }

    if pages.is_empty() || !current_body_items.is_empty() {
        push_pdf_page(
            document,
            &mut pages,
            current_section_index,
            current_body_items,
        );
    }

    Ok(pages)
}

fn flush_pdf_page_if_section_changes(
    document: &Document,
    pages: &mut Vec<PdfPage>,
    current_section_index: &mut usize,
    current_body_items: &mut Vec<PdfPageBodyItem>,
    current_body_height: &mut f32,
    next_section_index: usize,
) {
    if !current_body_items.is_empty() && next_section_index != *current_section_index {
        push_pdf_page(
            document,
            pages,
            *current_section_index,
            std::mem::take(current_body_items),
        );
        *current_body_height = 0.0;
    }
    *current_section_index = next_section_index;
}

fn push_pdf_body_item(
    document: &Document,
    pages: &mut Vec<PdfPage>,
    section_index: usize,
    current_body_items: &mut Vec<PdfPageBodyItem>,
    current_body_height: &mut f32,
    item: PdfPageBodyItem,
) {
    let section_page_number = pages
        .iter()
        .filter(|page| page.section_index == section_index)
        .count()
        + 1;
    let body_height = pdf_body_height_points(
        document,
        section_index,
        section_page_number,
        pages.len() + 1,
    );
    if item.height() > body_height {
        for chunk in split_oversized_pdf_body_item(item, body_height) {
            push_pdf_body_item(
                document,
                pages,
                section_index,
                current_body_items,
                current_body_height,
                chunk,
            );
        }
        return;
    }
    let item_height = item.height();
    if !current_body_items.is_empty() && *current_body_height + item_height > body_height {
        push_pdf_page(
            document,
            pages,
            section_index,
            std::mem::take(current_body_items),
        );
        *current_body_height = 0.0;
    }
    *current_body_height += item_height;
    current_body_items.push(item);
}

fn split_oversized_pdf_body_item(item: PdfPageBodyItem, body_height: f32) -> Vec<PdfPageBodyItem> {
    match item {
        PdfPageBodyItem::TableRow(row) => split_oversized_pdf_table_row(row, body_height)
            .into_iter()
            .map(PdfPageBodyItem::TableRow)
            .collect(),
        PdfPageBodyItem::Figure(figure) => split_oversized_pdf_figure(figure, body_height)
            .into_iter()
            .map(PdfPageBodyItem::Figure)
            .collect(),
        PdfPageBodyItem::TextLine(line) => vec![PdfPageBodyItem::TextLine(line)],
    }
}

fn split_oversized_pdf_table_row(
    row: PdfTableRowLayout,
    body_height: f32,
) -> Vec<PdfTableRowLayout> {
    let usable_text_height =
        (body_height - PDF_TABLE_ROW_GAP_POINTS - PDF_TABLE_CELL_PADDING * 2.0)
            .max(PDF_TABLE_LEADING);
    let max_lines = (usable_text_height / PDF_TABLE_LEADING).floor().max(1.0) as usize;
    let total_lines = row
        .cells
        .iter()
        .map(|cell| cell.lines.len())
        .max()
        .unwrap_or(1);
    if total_lines <= max_lines {
        return vec![row];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < total_lines {
        let end = (start + max_lines).min(total_lines);
        let cells = row
            .cells
            .iter()
            .map(|cell| {
                let lines = if start < cell.lines.len() {
                    cell.lines[start..end.min(cell.lines.len())].to_vec()
                } else {
                    vec![empty_pdf_linked_line()]
                };
                PdfTableCellLayout {
                    lines,
                    presentation: cell.presentation.clone(),
                }
            })
            .collect::<Vec<_>>();
        let max_chunk_lines = cells.iter().map(|cell| cell.lines.len()).max().unwrap_or(1);
        let row_height = max_chunk_lines as f32 * PDF_TABLE_LEADING + PDF_TABLE_CELL_PADDING * 2.0;
        chunks.push(PdfTableRowLayout {
            cells,
            cell_widths: row.cell_widths.clone(),
            row_height,
            block_height: row_height + PDF_TABLE_ROW_GAP_POINTS,
        });
        start = end;
    }
    chunks
}

fn split_oversized_pdf_figure(
    mut figure: PdfFigureLayout,
    body_height: f32,
) -> Vec<PdfFigureLayout> {
    let max_box_height = (body_height - PDF_FIGURE_GAP_POINTS).max(PDF_FIGURE_MIN_HEIGHT);
    let usable_text_height = (max_box_height - PDF_FIGURE_PADDING * 2.0).max(PDF_FIGURE_LEADING);
    let max_lines = (usable_text_height / PDF_FIGURE_LEADING).floor().max(1.0) as usize;
    if figure.lines.len() <= max_lines && figure.box_height <= max_box_height {
        return vec![figure];
    }
    if let Some(image) = figure.image.take() {
        return split_oversized_pdf_image_figure(figure, image, max_box_height);
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < figure.lines.len() {
        let end = (start + max_lines).min(figure.lines.len());
        let lines = figure.lines[start..end].to_vec();
        let text_height = lines.len() as f32 * PDF_FIGURE_LEADING + PDF_FIGURE_PADDING * 2.0;
        let box_height = figure.box_height.min(max_box_height).max(text_height);
        chunks.push(PdfFigureLayout {
            lines,
            alignment: figure.alignment,
            width: figure.width,
            box_height,
            block_height: box_height + PDF_FIGURE_GAP_POINTS,
            image: None,
        });
        start = end;
    }
    if chunks.is_empty() {
        let box_height = figure.box_height.min(max_box_height);
        chunks.push(PdfFigureLayout {
            lines: vec![String::new()],
            alignment: figure.alignment,
            width: figure.width,
            box_height,
            block_height: box_height + PDF_FIGURE_GAP_POINTS,
            image: None,
        });
    }
    chunks
}

fn split_oversized_pdf_image_figure(
    figure: PdfFigureLayout,
    mut image: PdfFigureImageLayout,
    max_box_height: f32,
) -> Vec<PdfFigureLayout> {
    let inner_height_limit = (max_box_height - PDF_FIGURE_PADDING * 2.0).max(PDF_FIGURE_LEADING);
    let reserved_text_height = if figure.lines.is_empty() {
        0.0
    } else {
        PDF_FIGURE_IMAGE_TEXT_GAP_POINTS + PDF_FIGURE_LEADING
    };
    let image_height_limit = (inner_height_limit - reserved_text_height).max(PDF_FIGURE_LEADING);
    if image.height > image_height_limit {
        let scale = image_height_limit / image.height;
        image.height *= scale;
        image.width = (image.width * scale).min((figure.width - PDF_FIGURE_PADDING * 2.0).max(1.0));
    }

    let text_gap = if figure.lines.is_empty() {
        0.0
    } else {
        PDF_FIGURE_IMAGE_TEXT_GAP_POINTS
    };
    let first_text_capacity = (inner_height_limit - image.height - text_gap).max(0.0);
    let first_line_count =
        ((first_text_capacity / PDF_FIGURE_LEADING).floor() as usize).min(figure.lines.len());
    let first_lines = figure.lines[..first_line_count].to_vec();
    let first_text_height = if first_lines.is_empty() {
        0.0
    } else {
        PDF_FIGURE_IMAGE_TEXT_GAP_POINTS + first_lines.len() as f32 * PDF_FIGURE_LEADING
    };
    let first_box_height = (PDF_FIGURE_PADDING * 2.0 + image.height + first_text_height)
        .max(PDF_FIGURE_MIN_HEIGHT)
        .min(max_box_height);

    let mut chunks = vec![PdfFigureLayout {
        lines: first_lines,
        alignment: figure.alignment,
        width: figure.width,
        box_height: first_box_height,
        block_height: first_box_height + PDF_FIGURE_GAP_POINTS,
        image: Some(image),
    }];

    let text_only_capacity = ((max_box_height - PDF_FIGURE_PADDING * 2.0).max(PDF_FIGURE_LEADING)
        / PDF_FIGURE_LEADING)
        .floor()
        .max(1.0) as usize;
    let mut start = first_line_count;
    while start < figure.lines.len() {
        let end = (start + text_only_capacity).min(figure.lines.len());
        let lines = figure.lines[start..end].to_vec();
        let box_height = (PDF_FIGURE_PADDING * 2.0 + lines.len() as f32 * PDF_FIGURE_LEADING)
            .max(PDF_FIGURE_MIN_HEIGHT)
            .min(max_box_height);
        chunks.push(PdfFigureLayout {
            lines,
            alignment: figure.alignment,
            width: figure.width,
            box_height,
            block_height: box_height + PDF_FIGURE_GAP_POINTS,
            image: None,
        });
        start = end;
    }
    chunks
}

fn push_pdf_page(
    document: &Document,
    pages: &mut Vec<PdfPage>,
    section_index: usize,
    body_items: Vec<PdfPageBodyItem>,
) {
    let section_page_number = pages
        .iter()
        .filter(|page| page.section_index == section_index)
        .count()
        + 1;
    pages.push(PdfPage {
        page_number: pages.len() + 1,
        section_index,
        section_page_number,
        page_setup: document.sections[section_index].page.clone(),
        body_items,
    });
}

fn select_pdf_pages(
    pages: &[PdfPage],
    range: Option<PdfPageRange>,
) -> Result<&[PdfPage], ExportError> {
    let Some(range) = range else {
        return Ok(pages);
    };
    if range.start == 0 || range.end < range.start || range.end > pages.len() {
        return Err(ExportError::InvalidPdfPageRange);
    }
    Ok(&pages[(range.start - 1)..range.end])
}

fn pdf_flow_items(document: &Document) -> Result<Vec<PdfFlowItem>, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let mut items = Vec::new();
    let bookmark_id_counts = collect_pdf_safe_bookmark_id_counts(&document.sections);
    let note_references = collect_pdf_ordered_note_references(document);
    let active_note_ids = note_references
        .iter()
        .map(|reference| reference.id.clone())
        .collect::<BTreeSet<_>>();
    let mut state = PdfProjectionState {
        bookmark_id_counts: &bookmark_id_counts,
        active_note_ids: &active_note_ids,
        emitted_note_reference_ids: BTreeSet::new(),
    };
    for (section_index, section) in document.sections.iter().enumerate() {
        for block in &section.blocks {
            push_block_pdf_items(block, document, section_index, &mut state, &mut items);
        }
    }

    let note_section_index = document.sections.len().saturating_sub(1);
    push_pdf_notes_items(document, &note_references, note_section_index, &mut items);

    if items.is_empty() {
        items.push(PdfFlowItem::Text(PdfProjectedText {
            section_index: 0,
            text: PdfLinkedText::default(),
        }));
    }

    Ok(items)
}

fn push_block_pdf_items(
    block: &Block,
    document: &Document,
    section_index: usize,
    state: &mut PdfProjectionState<'_>,
    items: &mut Vec<PdfFlowItem>,
) {
    match block {
        Block::PageBreak => items.push(PdfFlowItem::PageBreak { section_index }),
        Block::Table(table) => {
            let column_widths = table.sanitized_column_widths().unwrap_or_default();
            for row in &table.rows {
                let mut cells = Vec::new();
                for cell in &row.cells {
                    let mut cell_text = PdfLinkedText::default();
                    for block in &cell.blocks {
                        cell_text.append(&pdf_block_linked_text(block, document, state));
                    }
                    cells.push(PdfProjectedTableCell {
                        text: cell_text.trimmed(),
                        presentation: cell.presentation.clone(),
                    });
                }
                items.push(PdfFlowItem::TableRow(PdfProjectedTableRow {
                    section_index,
                    column_widths: column_widths.clone(),
                    cells,
                }));
            }
            items.push(PdfFlowItem::Text(PdfProjectedText {
                section_index,
                text: PdfLinkedText::default(),
            }));
        }
        Block::Image(image) => {
            items.push(PdfFlowItem::Figure(PdfProjectedFigure {
                section_index,
                alt_text: image
                    .alt_text
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                caption: image
                    .presentation
                    .caption
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                alignment: image.presentation.alignment,
                scale_percent: image.presentation.scale_percent,
                jpeg: pdf_jpeg_image_candidate(document, image),
            }));
            items.push(PdfFlowItem::Text(PdfProjectedText {
                section_index,
                text: PdfLinkedText::default(),
            }));
        }
        _ => {
            let text = pdf_block_linked_text(block, document, state);
            for line in text.split_lines() {
                items.push(PdfFlowItem::Text(PdfProjectedText {
                    section_index,
                    text: line,
                }));
            }
            items.push(PdfFlowItem::Text(PdfProjectedText {
                section_index,
                text: PdfLinkedText::default(),
            }));
        }
    }
}

fn pdf_block_linked_text(
    block: &Block,
    document: &Document,
    state: &mut PdfProjectionState<'_>,
) -> PdfLinkedText {
    let mut output = PdfLinkedText::default();
    match block {
        Block::Paragraph(paragraph) => {
            push_pdf_bookmark_destination(paragraph.bookmark_id.as_deref(), state, &mut output);
            push_inlines_pdf_linked_text(&paragraph.inlines, document, state, &mut output)
        }
        Block::Heading(heading) => {
            push_pdf_bookmark_destination(heading.bookmark_id.as_deref(), state, &mut output);
            push_inlines_pdf_linked_text(&heading.inlines, document, state, &mut output)
        }
        Block::TableOfContents(table_of_contents) => {
            push_table_of_contents_pdf_linked_text(table_of_contents, &mut output);
        }
        Block::List(list) => {
            let ordered = document
                .lists
                .get(&list.definition_id)
                .map(|definition| definition.ordered)
                .unwrap_or(false);
            for (index, item) in list.items.iter().enumerate() {
                for _ in 0..item.level {
                    output.push_plain("  ");
                }
                if ordered {
                    output.push_plain(&(index + 1).to_string());
                    output.push_plain(". ");
                } else {
                    output.push_plain("- ");
                }
                for block in &item.blocks {
                    output.append(&pdf_block_linked_text(block, document, state));
                    output.push_plain("\n");
                }
            }
        }
        Block::Table(table) => {
            for row in &table.rows {
                let mut first_cell = true;
                for cell in &row.cells {
                    if !first_cell {
                        output.push_plain("    ");
                    }
                    first_cell = false;
                    let mut cell_text = PdfLinkedText::default();
                    for block in &cell.blocks {
                        cell_text.append(&pdf_block_linked_text(block, document, state));
                    }
                    output.append(&cell_text.trimmed());
                }
                output.push_plain("\n");
            }
        }
        Block::Image(image) => {
            if let Some(alt_text) = image
                .alt_text
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                output.push_plain(alt_text);
            }
            if let Some(caption) = image
                .presentation
                .caption
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                if !output.is_empty() {
                    output.push_plain("\n");
                }
                output.push_plain(caption);
            }
        }
        Block::PageBreak => {}
    }
    output
}

fn push_inlines_pdf_text(inlines: &[Inline], document: &Document, output: &mut String) {
    for inline in inlines {
        output.push_str(&inline_pdf_text(inline, document));
    }
}

fn push_inlines_pdf_linked_text(
    inlines: &[Inline],
    document: &Document,
    state: &mut PdfProjectionState<'_>,
    output: &mut PdfLinkedText,
) {
    for inline in inlines {
        if let Some(reference) = inline
            .note_reference
            .as_ref()
            .and_then(|reference| validate_note_reference(reference).ok())
        {
            if state.active_note_ids.contains(&reference.id)
                && document
                    .notes
                    .get(&reference.id)
                    .is_some_and(|note| note.kind == reference.kind)
            {
                if state
                    .emitted_note_reference_ids
                    .insert(reference.id.clone())
                {
                    output.push_destination_marker(PdfDestinationId::NoteReference(
                        reference.id.clone(),
                    ));
                }
                output.push_destination_link(
                    &reference.label,
                    PdfDestinationId::NoteBody(reference.id.clone()),
                );
                continue;
            }
        }
        let text = inline_pdf_text(inline, document);
        match inline.link.as_deref().and_then(sanitize_pdf_link_target) {
            Some(PdfLinkTarget::Uri(uri)) => output.push_uri(&text, &uri),
            Some(PdfLinkTarget::Destination(destination_id)) => {
                output.push_destination_link(&text, destination_id)
            }
            None => output.push_plain(&text),
        }
    }
}

fn inline_pdf_text(inline: &Inline, document: &Document) -> String {
    if let Some(reference) = inline.note_reference.as_ref() {
        return reference.label.clone();
    }
    match inline.field {
        Some(PageField::PageNumber) => PDF_PAGE_NUMBER_TOKEN.to_string(),
        Some(PageField::PageCount) => PDF_PAGE_COUNT_TOKEN.to_string(),
        Some(PageField::Date) => document.meta.modified_at.format("%Y-%m-%d").to_string(),
        None => inline.text.clone(),
    }
}

fn pdf_body_height_points(
    document: &Document,
    section_index: usize,
    section_page_number: usize,
    next_global_page_number: usize,
) -> f32 {
    let (top, bottom) = pdf_body_vertical_bounds(
        document,
        section_index,
        section_page_number,
        next_global_page_number,
        1,
    );
    (top - bottom).max(PDF_LEADING)
}

fn pdf_body_vertical_bounds(
    document: &Document,
    section_index: usize,
    section_page_number: usize,
    page_number: usize,
    total_pages: usize,
) -> (f32, f32) {
    let page_setup = &document.sections[section_index].page;
    let page_height = mm_to_points(page_setup.height_mm);
    let margin_top = mm_to_points(page_setup.margin_top_mm).max(PDF_MIN_MARGIN_POINTS);
    let margin_bottom = mm_to_points(page_setup.margin_bottom_mm).max(PDF_MIN_MARGIN_POINTS);
    let header_lines = pdf_header_lines(
        document,
        section_index,
        section_page_number,
        page_number,
        total_pages,
    )
    .len();
    let footer_lines = pdf_footer_lines(
        document,
        section_index,
        section_page_number,
        page_number,
        total_pages,
    )
    .len();
    let header_height = pdf_region_height(header_lines);
    let footer_height = pdf_region_height(footer_lines);
    let top = page_height - margin_top - header_height - pdf_region_gap(header_lines);
    let bottom = margin_bottom + footer_height + pdf_region_gap(footer_lines);
    (top, bottom)
}

fn pdf_content_width(page_setup: &PageSetup) -> f32 {
    let page_width = mm_to_points(page_setup.width_mm);
    let margin_left = mm_to_points(page_setup.margin_left_mm).max(PDF_MIN_MARGIN_POINTS);
    let margin_right = mm_to_points(page_setup.margin_right_mm).max(PDF_MIN_MARGIN_POINTS);
    (page_width - margin_left - margin_right).max(PDF_FONT_SIZE * 20.0)
}

fn pdf_wrap_char_limit(page_setup: &PageSetup) -> usize {
    pdf_wrap_char_limit_for_width(pdf_content_width(page_setup), PDF_FONT_SIZE, 20)
}

fn pdf_wrap_char_limit_for_width(width: f32, font_size: f32, minimum: usize) -> usize {
    (width / (font_size * PDF_TEXT_WIDTH_FACTOR))
        .floor()
        .max(minimum as f32) as usize
}

fn pdf_region_height(line_count: usize) -> f32 {
    if line_count == 0 {
        0.0
    } else {
        line_count as f32 * PDF_REGION_LEADING
    }
}

fn pdf_region_gap(line_count: usize) -> f32 {
    if line_count == 0 {
        0.0
    } else {
        PDF_REGION_GAP_POINTS
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct HtmlExportOptions {
    print_ready: bool,
}

fn export_html_with_options(
    document: &Document,
    options: HtmlExportOptions,
) -> Result<String, ExportError> {
    if document.sections.is_empty() {
        return Err(ExportError::EmptyDocument);
    }

    let mut output = String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\">");
    output.push_str(
        "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'\">",
    );
    output.push_str("<meta name=\"generator\" content=\"900Word\"><title>");
    output.push_str(&escape_html(&document.meta.title));
    output.push_str("</title>");
    push_export_css(document, options, &mut output);
    output.push_str("</head><body>");

    for section in &document.sections {
        output.push_str("<section>");
        push_section_regions_html(section, document, true, &mut output);
        for block in &section.blocks {
            push_block_html(block, document, &mut output);
        }
        push_section_regions_html(section, document, false, &mut output);
        output.push_str("</section>");
    }
    push_notes_html(document, &mut output);

    output.push_str("</body></html>");
    Ok(output)
}

fn push_section_regions_text(
    section: &Section,
    document: &Document,
    before_body: bool,
    output: &mut String,
) {
    if before_body {
        if section.page_regions.different_first_page {
            push_region_text(
                "First page header",
                &section.page_regions.first_header,
                document,
                output,
            );
        }
        push_region_text("Header", &section.page_regions.header, document, output);
    } else {
        push_region_text("Footer", &section.page_regions.footer, document, output);
        if section.page_regions.different_first_page {
            push_region_text(
                "First page footer",
                &section.page_regions.first_footer,
                document,
                output,
            );
        }
    }
}

fn push_region_text(label: &str, region: &PageRegion, document: &Document, output: &mut String) {
    if region.blocks.is_empty() {
        return;
    }
    output.push('[');
    output.push_str(label);
    output.push_str("]\n");
    for block in &region.blocks {
        push_page_region_block_text(block, document, output);
        output.push('\n');
    }
}

fn push_page_region_block_text(block: &PageRegionBlock, document: &Document, output: &mut String) {
    match block {
        PageRegionBlock::Paragraph(paragraph) => {
            for inline in &paragraph.inlines {
                output.push_str(&inline_export_text(inline, document));
            }
        }
    }
}

fn push_block_text(block: &Block, document: &Document, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => push_paragraph_text(paragraph, document, output),
        Block::Heading(heading) => push_heading_text(heading, document, output),
        Block::TableOfContents(table_of_contents) => {
            push_table_of_contents_text(table_of_contents, output)
        }
        Block::List(list) => {
            for item in &list.items {
                for block in &item.blocks {
                    push_block_text(block, document, output);
                    output.push('\n');
                }
            }
        }
        Block::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    for block in &cell.blocks {
                        push_block_text(block, document, output);
                    }
                    output.push('\t');
                }
                output.push('\n');
            }
        }
        Block::Image(image) => {
            if let Some(alt_text) = image
                .alt_text
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                output.push_str(alt_text);
            }
            if let Some(caption) = image
                .presentation
                .caption
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                if image
                    .alt_text
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    output.push('\n');
                }
                output.push_str(caption);
            }
        }
        Block::PageBreak => output.push_str("\n--- page break ---\n"),
    }
}

fn push_paragraph_text(paragraph: &Paragraph, document: &Document, output: &mut String) {
    for inline in &paragraph.inlines {
        output.push_str(&inline_export_text(inline, document));
    }
}

fn push_heading_text(heading: &Heading, document: &Document, output: &mut String) {
    for inline in &heading.inlines {
        output.push_str(&inline_export_text(inline, document));
    }
}

fn push_table_of_contents_text(table_of_contents: &TableOfContents, output: &mut String) {
    let title = table_of_contents.title.trim();
    if !title.is_empty() {
        output.push_str(title);
    }
    for entry in &table_of_contents.entries {
        if !output.is_empty() {
            output.push('\n');
        }
        for _ in 1..entry.level.clamp(1, 3) {
            output.push_str("  ");
        }
        output.push_str(&entry.text);
    }
}

fn push_table_of_contents_pdf_linked_text(
    table_of_contents: &TableOfContents,
    output: &mut PdfLinkedText,
) {
    let title = table_of_contents.title.trim();
    if !title.is_empty() {
        output.push_plain(title);
    }
    for entry in &table_of_contents.entries {
        if !output.is_empty() {
            output.push_plain("\n");
        }
        for _ in 1..entry.level.clamp(1, 3) {
            output.push_plain("  ");
        }
        if let Some(target) = sanitize_bookmark_id(&entry.target_bookmark_id) {
            output
                .push_destination_link(&entry.text, PdfDestinationId::Bookmark(target.to_string()));
        } else {
            output.push_plain(&entry.text);
        }
    }
}

fn push_pdf_bookmark_destination(
    bookmark_id: Option<&str>,
    state: &PdfProjectionState<'_>,
    output: &mut PdfLinkedText,
) {
    let Some(bookmark_id) = bookmark_id.and_then(sanitize_bookmark_id) else {
        return;
    };
    if state.bookmark_id_counts.get(bookmark_id).copied() == Some(1) {
        output.push_destination_marker(PdfDestinationId::Bookmark(bookmark_id.to_string()));
    }
}

fn collect_pdf_safe_bookmark_id_counts(sections: &[Section]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for section in sections {
        collect_pdf_safe_bookmark_id_counts_from_blocks(&section.blocks, &mut counts);
    }
    counts
}

fn collect_pdf_safe_bookmark_id_counts_from_blocks(
    blocks: &[Block],
    counts: &mut BTreeMap<String, usize>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                if let Some(bookmark_id) = paragraph
                    .bookmark_id
                    .as_deref()
                    .and_then(sanitize_bookmark_id)
                {
                    *counts.entry(bookmark_id.to_string()).or_insert(0) += 1;
                }
            }
            Block::Heading(heading) => {
                if let Some(bookmark_id) = heading
                    .bookmark_id
                    .as_deref()
                    .and_then(sanitize_bookmark_id)
                {
                    *counts.entry(bookmark_id.to_string()).or_insert(0) += 1;
                }
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_pdf_safe_bookmark_id_counts_from_blocks(&item.blocks, counts);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_pdf_safe_bookmark_id_counts_from_blocks(&cell.blocks, counts);
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_pdf_ordered_note_references(document: &Document) -> Vec<word_core::InlineNoteReference> {
    let mut references = Vec::new();
    let mut seen = BTreeSet::new();
    for section in &document.sections {
        collect_pdf_note_references_from_blocks(
            document,
            &section.blocks,
            &mut references,
            &mut seen,
        );
    }
    references
}

fn collect_pdf_note_references_from_blocks(
    document: &Document,
    blocks: &[Block],
    references: &mut Vec<word_core::InlineNoteReference>,
    seen: &mut BTreeSet<String>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => collect_pdf_note_references_from_inlines(
                document,
                &paragraph.inlines,
                references,
                seen,
            ),
            Block::Heading(heading) => collect_pdf_note_references_from_inlines(
                document,
                &heading.inlines,
                references,
                seen,
            ),
            Block::List(list) => {
                for item in &list.items {
                    collect_pdf_note_references_from_blocks(
                        document,
                        &item.blocks,
                        references,
                        seen,
                    );
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_pdf_note_references_from_blocks(
                            document,
                            &cell.blocks,
                            references,
                            seen,
                        );
                    }
                }
            }
            Block::TableOfContents(_) | Block::Image(_) | Block::PageBreak => {}
        }
    }
}

fn collect_pdf_note_references_from_inlines(
    document: &Document,
    inlines: &[Inline],
    references: &mut Vec<word_core::InlineNoteReference>,
    seen: &mut BTreeSet<String>,
) {
    for inline in inlines {
        let Some(reference) = inline
            .note_reference
            .as_ref()
            .and_then(|reference| validate_note_reference(reference).ok())
        else {
            continue;
        };
        if document
            .notes
            .get(&reference.id)
            .is_some_and(|note| note.kind == reference.kind)
            && seen.insert(reference.id.clone())
        {
            references.push(reference);
        }
    }
}

fn push_pdf_notes_items(
    document: &Document,
    references: &[word_core::InlineNoteReference],
    section_index: usize,
    items: &mut Vec<PdfFlowItem>,
) {
    push_pdf_note_kind_items(
        document,
        references,
        NoteKind::Footnote,
        "Footnotes",
        section_index,
        items,
    );
    push_pdf_note_kind_items(
        document,
        references,
        NoteKind::Endnote,
        "Endnotes",
        section_index,
        items,
    );
}

fn push_pdf_note_kind_items(
    document: &Document,
    references: &[word_core::InlineNoteReference],
    kind: NoteKind,
    title: &str,
    section_index: usize,
    items: &mut Vec<PdfFlowItem>,
) {
    let rows = references
        .iter()
        .filter(|reference| reference.kind == kind)
        .filter_map(|reference| {
            let note = document.notes.get(&reference.id)?;
            if note.kind != reference.kind {
                return None;
            }
            let mut row = PdfLinkedText::default();
            row.push_destination_marker(PdfDestinationId::NoteBody(reference.id.clone()));
            row.push_destination_link(
                &format!("[{}]", reference.label),
                PdfDestinationId::NoteReference(reference.id.clone()),
            );
            row.push_plain(" ");
            row.push_plain(&note.body);
            Some(row)
        })
        .collect::<Vec<_>>();

    if rows.is_empty() {
        return;
    }

    if !items.is_empty() {
        items.push(PdfFlowItem::Text(PdfProjectedText {
            section_index,
            text: PdfLinkedText::default(),
        }));
    }
    items.push(PdfFlowItem::Text(PdfProjectedText {
        section_index,
        text: PdfLinkedText::from_plain(title),
    }));
    for row in rows {
        items.push(PdfFlowItem::Text(PdfProjectedText {
            section_index,
            text: row,
        }));
    }
}

fn push_notes_text(document: &Document, output: &mut String) {
    let references = collect_ordered_note_references(&document.sections);
    push_note_kind_text(
        document,
        &references,
        NoteKind::Footnote,
        "Footnotes",
        output,
    );
    push_note_kind_text(document, &references, NoteKind::Endnote, "Endnotes", output);
}

fn push_note_kind_text(
    document: &Document,
    references: &[word_core::InlineNoteReference],
    kind: NoteKind,
    title: &str,
    output: &mut String,
) {
    let mut lines = Vec::new();
    for reference in references.iter().filter(|reference| reference.kind == kind) {
        let Some(note) = document.notes.get(&reference.id) else {
            continue;
        };
        if note.kind != reference.kind {
            continue;
        }
        lines.push(format!("[{}] {}", reference.label, note.body));
    }
    if lines.is_empty() {
        return;
    }
    if !output.trim_end().is_empty() {
        output.push('\n');
    }
    output.push('\n');
    output.push_str(title);
    output.push('\n');
    output.push_str(&lines.join("\n"));
    output.push('\n');
}

fn push_section_regions_html(
    section: &Section,
    document: &Document,
    before_body: bool,
    output: &mut String,
) {
    if before_body {
        if section.page_regions.different_first_page {
            push_region_html(
                "first-header",
                &section.page_regions.first_header,
                document,
                output,
            );
        }
        push_region_html("header", &section.page_regions.header, document, output);
    } else {
        push_region_html("footer", &section.page_regions.footer, document, output);
        if section.page_regions.different_first_page {
            push_region_html(
                "first-footer",
                &section.page_regions.first_footer,
                document,
                output,
            );
        }
    }
}

fn push_region_html(kind: &str, region: &PageRegion, document: &Document, output: &mut String) {
    if region.blocks.is_empty() {
        return;
    }
    let tag = if kind.ends_with("footer") {
        "footer"
    } else {
        "header"
    };
    output.push('<');
    output.push_str(tag);
    output.push_str(" data-page-region=\"");
    output.push_str(kind);
    output.push_str("\">");
    for block in &region.blocks {
        match block {
            PageRegionBlock::Paragraph(paragraph) => {
                output.push_str("<p>");
                push_inlines_html(&paragraph.inlines, document, output);
                output.push_str("</p>");
            }
        }
    }
    output.push_str("</");
    output.push_str(tag);
    output.push('>');
}

fn push_block_html(block: &Block, document: &Document, output: &mut String) {
    match block {
        Block::Paragraph(paragraph) => {
            output.push_str("<p");
            output.push_str(&paragraph_html_attrs(paragraph, &document.styles));
            output.push('>');
            push_inlines_html(&paragraph.inlines, document, output);
            output.push_str("</p>");
        }
        Block::Heading(heading) => {
            let level = heading.level.clamp(1, 6);
            output.push_str(&format!("<h{level}"));
            output.push_str(&bookmark_html_attr(heading.bookmark_id.as_deref()));
            output.push('>');
            push_inlines_html(&heading.inlines, document, output);
            output.push_str(&format!("</h{level}>"));
        }
        Block::TableOfContents(table_of_contents) => {
            push_table_of_contents_html(table_of_contents, output);
        }
        Block::List(list) => {
            let tag = if document
                .lists
                .get(&list.definition_id)
                .map(|definition| definition.ordered)
                .unwrap_or(false)
            {
                "ol"
            } else {
                "ul"
            };
            output.push('<');
            output.push_str(tag);
            output.push('>');
            for item in &list.items {
                output.push_str("<li>");
                for block in &item.blocks {
                    push_block_html(block, document, output);
                }
                output.push_str("</li>");
            }
            output.push_str("</");
            output.push_str(tag);
            output.push('>');
        }
        Block::Table(table) => {
            output.push_str("<table");
            output.push_str(&table_column_widths_html_attr(table));
            output.push('>');
            push_table_colgroup_html(table, output);
            for row in &table.rows {
                output.push_str("<tr>");
                for cell in &row.cells {
                    output.push_str("<td");
                    output.push_str(&table_cell_html_attrs(&cell.presentation));
                    output.push('>');
                    for block in &cell.blocks {
                        push_block_html(block, document, output);
                    }
                    output.push_str("</td>");
                }
                output.push_str("</tr>");
            }
            output.push_str("</table>");
        }
        Block::Image(image) => {
            let alt_text = image.alt_text.as_deref().unwrap_or("Image");
            let caption = image
                .presentation
                .caption
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(alt_text);
            output.push_str("<figure data-asset=\"");
            output.push_str(&escape_html(&image.asset_id));
            output.push('"');
            output.push_str(&image_html_attrs(image));
            output.push('>');
            if let Some(data_url) = image_data_url(document, &image.asset_id) {
                output.push_str("<img src=\"");
                output.push_str(&data_url);
                output.push_str("\" alt=\"");
                output.push_str(&escape_html(alt_text));
                output.push_str("\">");
            }
            output.push_str("<figcaption>");
            output.push_str(&escape_html(caption));
            output.push_str("</figcaption>");
            output.push_str("</figure>");
        }
        Block::PageBreak => output.push_str("<hr data-page-break=\"true\">"),
    }
}

fn push_table_of_contents_html(table_of_contents: &TableOfContents, output: &mut String) {
    output.push_str(
        "<nav data-900word-block=\"table-of-contents\" aria-label=\"Table of contents\">",
    );
    let title = table_of_contents.title.trim();
    if !title.is_empty() {
        output.push_str("<p class=\"toc-title\">");
        output.push_str(&escape_html(title));
        output.push_str("</p>");
    }
    output.push_str("<ol class=\"toc-list\">");
    for entry in &table_of_contents.entries {
        let Some(target) = sanitize_bookmark_id(&entry.target_bookmark_id) else {
            continue;
        };
        let level = entry.level.clamp(1, 3);
        output.push_str("<li data-toc-level=\"");
        output.push_str(&level.to_string());
        output.push_str("\"><a href=\"#");
        output.push_str(&escape_html(target));
        output.push_str("\">");
        output.push_str(&escape_html(&entry.text));
        output.push_str("</a></li>");
    }
    output.push_str("</ol></nav>");
}

fn table_column_widths_html_attr(table: &word_core::Table) -> String {
    let Some(widths) = table.sanitized_column_widths() else {
        return String::new();
    };
    let value = widths
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(" data-column-widths=\"{}\"", escape_html(&value))
}

fn push_table_colgroup_html(table: &word_core::Table, output: &mut String) {
    let Some(widths) = table.sanitized_column_widths() else {
        return;
    };
    output.push_str("<colgroup>");
    for width in widths {
        output.push_str("<col style=\"width:");
        output.push_str(&table_column_width_percent(width));
        output.push_str("%\">");
    }
    output.push_str("</colgroup>");
}

fn table_column_width_percent(width: u16) -> String {
    let integer = width / 10;
    let decimal = width % 10;
    if decimal == 0 {
        integer.to_string()
    } else {
        format!("{integer}.{decimal}")
    }
}

fn image_html_attrs(image: &ImageBlock) -> String {
    let mut attrs = String::new();
    let alignment = match image.presentation.alignment {
        word_core::ImageAlignment::Inline => "inline",
        word_core::ImageAlignment::Left => "left",
        word_core::ImageAlignment::Center => "center",
        word_core::ImageAlignment::Right => "right",
    };
    attrs.push_str(" data-align=\"");
    attrs.push_str(alignment);
    attrs.push('"');

    let scale = image.presentation.scale_percent.clamp(25, 200);
    attrs.push_str(" data-scale=\"");
    attrs.push_str(&scale.to_string());
    attrs.push('"');
    attrs.push_str(" style=\"");
    match image.presentation.alignment {
        word_core::ImageAlignment::Inline => {
            attrs.push_str("display:inline-block;");
        }
        word_core::ImageAlignment::Left => {
            attrs.push_str("margin-left:0;margin-right:auto;");
        }
        word_core::ImageAlignment::Center => {
            attrs.push_str("margin-left:auto;margin-right:auto;");
        }
        word_core::ImageAlignment::Right => {
            attrs.push_str("margin-left:auto;margin-right:0;");
        }
    }
    attrs.push_str("max-width:");
    attrs.push_str(&scale.to_string());
    attrs.push_str("%;\"");
    attrs
}

fn image_data_url(document: &Document, asset_id: &str) -> Option<String> {
    let asset = document.assets.get(asset_id)?;
    let detected = detect_image_media_type(&asset.bytes)?;
    if asset.byte_len != asset.bytes.len() || asset.media_type != detected {
        return None;
    }
    Some(format!(
        "data:{};base64,{}",
        detected,
        base64_encode(&asset.bytes)
    ))
}

fn detect_image_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn pdf_jpeg_image_candidate(
    document: &Document,
    image: &ImageBlock,
) -> Option<PdfJpegImageCandidate> {
    let asset = document.assets.get(&image.asset_id)?;
    let safe = pdf_safe_jpeg_asset(asset)?;
    Some(PdfJpegImageCandidate {
        asset_id: image.asset_id.clone(),
        width_px: safe.info.width_px,
        height_px: safe.info.height_px,
        components: safe.info.components,
    })
}

fn pdf_safe_jpeg_asset(asset: &AssetRef) -> Option<PdfSafeJpeg> {
    if asset.byte_len != asset.bytes.len()
        || asset.bytes.len() > PDF_MAX_JPEG_BYTES_PER_IMAGE
        || asset.media_type != "image/jpeg"
        || detect_image_media_type(&asset.bytes) != Some("image/jpeg")
    {
        return None;
    }
    sanitize_pdf_jpeg(&asset.bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PdfJpegInfo {
    width_px: u32,
    height_px: u32,
    components: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PdfSafeJpeg {
    info: PdfJpegInfo,
    bytes: Vec<u8>,
}

fn sanitize_pdf_jpeg(bytes: &[u8]) -> Option<PdfSafeJpeg> {
    if !bytes.starts_with(b"\xff\xd8") || !bytes.ends_with(b"\xff\xd9") {
        return None;
    }

    let mut sanitized = vec![0xff, 0xd8];
    let mut index = 2;
    let mut info = None;
    let mut saw_dqt = false;
    let mut saw_dht = false;
    while index < bytes.len() - 2 {
        if bytes[index] != 0xff {
            return None;
        }
        while index < bytes.len() - 2 && bytes[index] == 0xff {
            index += 1;
        }
        if index >= bytes.len() - 2 {
            return None;
        }
        let marker = bytes[index];
        index += 1;

        if marker == 0x00 {
            return None;
        }
        if jpeg_marker_without_payload(marker) {
            if marker == 0xd9 {
                return None;
            }
            sanitized.extend_from_slice(&[0xff, marker]);
            continue;
        }
        if index + 2 > bytes.len() {
            return None;
        }
        let segment_length = u16::from_be_bytes([bytes[index], bytes[index + 1]]) as usize;
        if segment_length < 2 {
            return None;
        }
        let segment_start = index + 2;
        let segment_end = index + segment_length;
        if segment_end > bytes.len() {
            return None;
        }

        if is_jpeg_metadata_marker(marker) {
            index = segment_end;
            continue;
        }
        if is_jpeg_sof_marker(marker) {
            if !is_supported_pdf_jpeg_sof_marker(marker) || info.is_some() {
                return None;
            }
            info = Some(parse_pdf_jpeg_sof_segment(
                &bytes[segment_start..segment_end],
            )?);
        } else if marker == 0xdb {
            saw_dqt = true;
        } else if marker == 0xc4 {
            saw_dht = true;
        }
        sanitized.extend_from_slice(&[0xff, marker]);
        sanitized.extend_from_slice(&bytes[index..segment_end]);
        if marker == 0xda {
            let info = info?;
            if !saw_dqt || !saw_dht {
                return None;
            }
            let entropy = &bytes[segment_end..];
            if !jpeg_entropy_data_is_metadata_free(entropy) {
                return None;
            }
            sanitized.extend_from_slice(entropy);
            return Some(PdfSafeJpeg {
                info,
                bytes: sanitized,
            });
        }
        index = segment_end;
    }
    None
}

fn parse_pdf_jpeg_sof_segment(segment: &[u8]) -> Option<PdfJpegInfo> {
    if segment.len() < 6 {
        return None;
    }
    let precision = segment[0];
    let height_px = u16::from_be_bytes([segment[1], segment[2]]) as u32;
    let width_px = u16::from_be_bytes([segment[3], segment[4]]) as u32;
    let components = segment[5];
    let expected_len = 6 + components as usize * 3;
    if precision != 8
        || width_px == 0
        || height_px == 0
        || width_px > PDF_MAX_JPEG_DIMENSION
        || height_px > PDF_MAX_JPEG_DIMENSION
        || width_px as u64 * height_px as u64 > PDF_MAX_JPEG_PIXELS
        || !matches!(components, 1 | 3)
        || segment.len() < expected_len
    {
        return None;
    }
    Some(PdfJpegInfo {
        width_px,
        height_px,
        components,
    })
}

fn jpeg_marker_without_payload(marker: u8) -> bool {
    marker == 0x01 || (0xd0..=0xd9).contains(&marker)
}

fn is_jpeg_metadata_marker(marker: u8) -> bool {
    (0xe0..=0xef).contains(&marker) || marker == 0xfe
}

fn is_jpeg_sof_marker(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

fn is_supported_pdf_jpeg_sof_marker(marker: u8) -> bool {
    marker == 0xc0
}

fn jpeg_entropy_data_is_metadata_free(bytes: &[u8]) -> bool {
    if !bytes.ends_with(b"\xff\xd9") {
        return false;
    }
    let mut index = 0;
    let mut saw_entropy = false;
    while index < bytes.len() {
        if bytes[index] != 0xff {
            saw_entropy = true;
            index += 1;
            continue;
        }
        if index + 1 >= bytes.len() {
            return false;
        }
        let marker = bytes[index + 1];
        match marker {
            0x00 => {
                saw_entropy = true;
                index += 2;
            }
            0xd0..=0xd7 => {
                index += 2;
            }
            0xd9 => {
                return saw_entropy && index + 2 == bytes.len();
            }
            0xff => {
                index += 1;
            }
            _ => return false,
        }
    }
    false
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        let value = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;
        output.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((value >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(value & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn push_inlines_html(inlines: &[Inline], document: &Document, output: &mut String) {
    for inline in inlines {
        push_inline_html(inline, document, output);
    }
}

fn push_inline_html(inline: &Inline, document: &Document, output: &mut String) {
    if let Some(reference) = inline.note_reference.as_ref() {
        output.push_str("<sup data-note-reference-id=\"");
        output.push_str(&escape_html(&reference.id));
        output.push_str("\" data-note-kind=\"");
        output.push_str(note_kind_name(reference.kind));
        output.push_str("\" data-note-label=\"");
        output.push_str(&escape_html(&reference.label));
        output.push_str("\">");
        output.push_str(&escape_html(&reference.label));
        output.push_str("</sup>");
        return;
    }

    if let Some(field) = inline.field {
        output.push_str("<span data-page-field=\"");
        output.push_str(page_field_name(field));
        output.push_str("\">");
        output.push_str(&escape_html(&inline_export_text(inline, document)));
        output.push_str("</span>");
        return;
    }

    let safe_href = inline.link.as_deref().and_then(sanitize_href);
    if let Some(href) = safe_href {
        output.push_str("<a rel=\"noreferrer\" href=\"");
        output.push_str(&escape_html(href));
        output.push_str("\">");
    }

    let styled_span = !inline.style.is_default();
    if styled_span {
        output.push_str("<span style=\"");
        if let Some(font_family) = inline.style.font_family.as_deref() {
            output.push_str("font-family:");
            output.push_str(&escape_html(font_family));
            output.push(';');
        }
        if let Some(font_size) = inline.style.font_size_pt {
            output.push_str("font-size:");
            output.push_str(&font_size.to_string());
            output.push_str("pt;");
        }
        if let Some(text_color) = inline.style.text_color.as_deref() {
            output.push_str("color:");
            output.push_str(&escape_html(text_color));
            output.push(';');
        }
        if let Some(highlight_color) = inline.style.highlight_color.as_deref() {
            output.push_str("background-color:");
            output.push_str(&escape_html(highlight_color));
            output.push(';');
        }
        output.push_str("\">");
    }

    let mut opened_marks = Vec::new();
    for mark in &inline.marks {
        let tag = mark_html_tag(*mark);
        output.push('<');
        output.push_str(tag);
        output.push('>');
        opened_marks.push(tag);
    }

    output.push_str(&escape_html(&inline_export_text(inline, document)));

    for tag in opened_marks.into_iter().rev() {
        output.push_str("</");
        output.push_str(tag);
        output.push('>');
    }

    if styled_span {
        output.push_str("</span>");
    }

    if safe_href.is_some() {
        output.push_str("</a>");
    }
}

fn inline_export_text(inline: &Inline, document: &Document) -> String {
    if let Some(reference) = inline.note_reference.as_ref() {
        return reference.label.clone();
    }
    match inline.field {
        Some(PageField::PageNumber) => "1".to_string(),
        Some(PageField::PageCount) => "1".to_string(),
        Some(PageField::Date) => document.meta.modified_at.format("%Y-%m-%d").to_string(),
        None => inline.text.clone(),
    }
}

fn push_notes_html(document: &Document, output: &mut String) {
    let references = collect_ordered_note_references(&document.sections);
    let mut sections = String::new();
    push_note_kind_html(
        document,
        &references,
        NoteKind::Footnote,
        "Footnotes",
        &mut sections,
    );
    push_note_kind_html(
        document,
        &references,
        NoteKind::Endnote,
        "Endnotes",
        &mut sections,
    );
    if sections.is_empty() {
        return;
    }
    output.push_str("<aside data-900word-notes=\"true\">");
    output.push_str(&sections);
    output.push_str("</aside>");
}

fn push_note_kind_html(
    document: &Document,
    references: &[word_core::InlineNoteReference],
    kind: NoteKind,
    title: &str,
    output: &mut String,
) {
    let mut rows = String::new();
    for reference in references.iter().filter(|reference| reference.kind == kind) {
        let Some(note) = document.notes.get(&reference.id) else {
            continue;
        };
        if note.kind != reference.kind {
            continue;
        }
        rows.push_str("<li><span class=\"note-label\">[");
        rows.push_str(&escape_html(&reference.label));
        rows.push_str("]</span> ");
        rows.push_str(&escape_html(&note.body));
        rows.push_str("</li>");
    }
    if rows.is_empty() {
        return;
    }
    output.push_str("<section data-note-kind=\"");
    output.push_str(note_kind_name(kind));
    output.push_str("\"><h2>");
    output.push_str(title);
    output.push_str("</h2><ol>");
    output.push_str(&rows);
    output.push_str("</ol></section>");
}

fn note_kind_name(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Footnote => "footnote",
        NoteKind::Endnote => "endnote",
    }
}

fn page_field_name(field: PageField) -> &'static str {
    match field {
        PageField::PageNumber => "page-number",
        PageField::PageCount => "page-count",
        PageField::Date => "date",
    }
}

fn table_cell_html_attrs(presentation: &TableCellPresentation) -> String {
    if presentation.is_default() {
        return String::new();
    }

    let mut attrs = String::new();
    let mut style = String::new();
    if let Some(background_color) = presentation
        .background_color
        .as_deref()
        .and_then(sanitize_table_cell_background_color)
    {
        attrs.push_str(" data-cell-background-color=\"");
        attrs.push_str(&escape_html(&background_color));
        attrs.push('"');
        style.push_str("background-color:");
        style.push_str(&background_color);
        style.push(';');
    }
    if let Some(alignment) = presentation.text_alignment {
        attrs.push_str(" data-cell-align=\"");
        attrs.push_str(paragraph_alignment_css(alignment));
        attrs.push('"');
        style.push_str("text-align:");
        style.push_str(paragraph_alignment_css(alignment));
        style.push(';');
    }
    if presentation.border == TableCellBorder::Hidden {
        attrs.push_str(" data-cell-border=\"hidden\"");
        style.push_str("border-color:transparent;");
    }
    if !style.is_empty() {
        attrs.push_str(" style=\"");
        attrs.push_str(&style);
        attrs.push('"');
    }
    attrs
}

fn paragraph_alignment_css(alignment: ParagraphAlignment) -> &'static str {
    match alignment {
        ParagraphAlignment::Left => "left",
        ParagraphAlignment::Center => "center",
        ParagraphAlignment::Right => "right",
        ParagraphAlignment::Justify => "justify",
    }
}

fn paragraph_html_attrs(
    paragraph: &Paragraph,
    styles: &BTreeMap<word_core::StyleId, Style>,
) -> String {
    let mut attrs = String::new();
    attrs.push_str(&bookmark_html_attr(paragraph.bookmark_id.as_deref()));
    attrs.push_str(" data-style=\"");
    attrs.push_str(&escape_html(paragraph.style.as_str()));
    attrs.push('"');
    let format = effective_paragraph_format(paragraph, styles);
    if format.is_default() {
        return attrs;
    }

    let mut style = String::new();
    if let Some(alignment) = format.alignment {
        let value = match alignment {
            word_core::ParagraphAlignment::Left => "left",
            word_core::ParagraphAlignment::Center => "center",
            word_core::ParagraphAlignment::Right => "right",
            word_core::ParagraphAlignment::Justify => "justify",
        };
        style.push_str("text-align:");
        style.push_str(value);
        style.push(';');
    }
    if let Some(line_spacing) = format.line_spacing_per_mille {
        style.push_str("line-height:");
        style.push_str(&(line_spacing as f32 / 1000.0).to_string());
        style.push(';');
    }
    if let Some(spacing_before) = format.spacing_before_mm {
        style.push_str("margin-top:");
        style.push_str(&spacing_before.to_string());
        style.push_str("mm;");
    }
    if let Some(spacing_after) = format.spacing_after_mm {
        style.push_str("margin-bottom:");
        style.push_str(&spacing_after.to_string());
        style.push_str("mm;");
    }
    if let Some(indent_start) = format.indent_start_mm {
        style.push_str("margin-left:");
        style.push_str(&indent_start.to_string());
        style.push_str("mm;");
    }
    if let Some(indent_end) = format.indent_end_mm {
        style.push_str("margin-right:");
        style.push_str(&indent_end.to_string());
        style.push_str("mm;");
    }
    if let Some(first_line_indent) = format.first_line_indent_mm {
        style.push_str("text-indent:");
        style.push_str(&first_line_indent.to_string());
        style.push_str("mm;");
    }
    if !style.is_empty() {
        attrs.push_str(" style=\"");
        attrs.push_str(&escape_html(&style));
        attrs.push('"');
    }
    attrs
}

fn effective_paragraph_format(
    paragraph: &Paragraph,
    styles: &BTreeMap<word_core::StyleId, Style>,
) -> ParagraphFormat {
    let mut format = styles
        .get(&paragraph.style)
        .filter(|style| style.kind == StyleKind::Paragraph)
        .and_then(|style| style.properties.paragraph.clone())
        .unwrap_or_default();

    if paragraph.format.alignment.is_some() {
        format.alignment = paragraph.format.alignment;
    }
    if paragraph.format.line_spacing_per_mille.is_some() {
        format.line_spacing_per_mille = paragraph.format.line_spacing_per_mille;
    }
    if paragraph.format.spacing_before_mm.is_some() {
        format.spacing_before_mm = paragraph.format.spacing_before_mm;
    }
    if paragraph.format.spacing_after_mm.is_some() {
        format.spacing_after_mm = paragraph.format.spacing_after_mm;
    }
    if paragraph.format.indent_start_mm.is_some() {
        format.indent_start_mm = paragraph.format.indent_start_mm;
    }
    if paragraph.format.indent_end_mm.is_some() {
        format.indent_end_mm = paragraph.format.indent_end_mm;
    }
    if paragraph.format.first_line_indent_mm.is_some() {
        format.first_line_indent_mm = paragraph.format.first_line_indent_mm;
    }
    format
}

fn mark_html_tag(mark: InlineMark) -> &'static str {
    match mark {
        InlineMark::Bold => "strong",
        InlineMark::Italic => "em",
        InlineMark::Underline => "u",
        InlineMark::Strikethrough => "s",
        InlineMark::Superscript => "sup",
        InlineMark::Subscript => "sub",
    }
}

fn sanitize_href(href: &str) -> Option<&str> {
    let trimmed = href.trim();
    if let Some(fragment) = trimmed.strip_prefix('#') {
        return sanitize_bookmark_id(fragment).map(|_| trimmed);
    }
    let lowercase = trimmed.to_ascii_lowercase();
    if lowercase.starts_with("https://")
        || lowercase.starts_with("http://")
        || lowercase.starts_with("mailto:")
    {
        Some(trimmed)
    } else {
        None
    }
}

fn sanitize_pdf_link_target(href: &str) -> Option<PdfLinkTarget> {
    let safe_href = sanitize_href(href)?;
    if let Some(fragment) = safe_href.strip_prefix('#') {
        return sanitize_bookmark_id(fragment)
            .map(str::to_string)
            .map(PdfDestinationId::Bookmark)
            .map(PdfLinkTarget::Destination);
    }
    if safe_href.len() > PDF_MAX_URI_BYTES
        || safe_href
            .chars()
            .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return None;
    }
    Some(PdfLinkTarget::Uri(safe_href.to_string()))
}

fn bookmark_html_attr(bookmark_id: Option<&str>) -> String {
    let Some(bookmark_id) = bookmark_id.and_then(sanitize_bookmark_id) else {
        return String::new();
    };
    format!(" id=\"{}\"", escape_html(bookmark_id))
}

fn sanitize_bookmark_id(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() || trimmed.len() > 64 {
        return None;
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Some(trimmed)
    } else {
        None
    }
}

fn push_export_css(document: &Document, options: HtmlExportOptions, output: &mut String) {
    let page = document
        .sections
        .first()
        .map(|section| &section.page)
        .cloned()
        .unwrap_or_default();
    output.push_str("<style>");
    output.push_str("body{font-family:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;line-height:1.55;color:#1d2433;background:#fff;margin:2rem;}");
    output.push_str("section{max-width:48rem;margin:0 auto;}table{border-collapse:collapse;width:100%;}td{border:1px solid #9aa7b8;padding:.35rem;vertical-align:top;}figure{margin:1rem 0;padding:.75rem;border:1px solid #d6dce5;}figcaption{color:#526070;}a{color:#0b63b6;}sup[data-note-reference-id]{color:#0f6b5f;font-size:.72em;font-weight:700;}aside[data-900word-notes]{max-width:48rem;margin:1.5rem auto 0;border-top:1px solid #cfd7df;padding-top:.75rem;}aside[data-900word-notes] h2{font-size:1rem;margin:.75rem 0 .35rem;}aside[data-900word-notes] ol{margin:.25rem 0 0;padding-left:1.25rem;}aside[data-900word-notes] li{margin:.25rem 0;}nav[data-900word-block=\"table-of-contents\"]{margin:1rem 0;padding:.75rem;border-left:3px solid #0f6b5f;background:#f3f7f5;}nav[data-900word-block=\"table-of-contents\"] .toc-title{margin:.1rem 0 .45rem;font-weight:700;}nav[data-900word-block=\"table-of-contents\"] ol{margin:.25rem 0 0;padding-left:1.25rem;}nav[data-900word-block=\"table-of-contents\"] li[data-toc-level=\"2\"]{margin-left:1rem;}nav[data-900word-block=\"table-of-contents\"] li[data-toc-level=\"3\"]{margin-left:2rem;}hr[data-page-break=\"true\"]{break-after:page;border:0;border-top:1px dashed #9aa7b8;}");
    if options.print_ready {
        output.push_str(&format!(
            "@page{{size:{}mm {}mm;margin:{}mm {}mm {}mm {}mm;}}body{{margin:0;}}section{{max-width:none;}}",
            page.width_mm,
            page.height_mm,
            page.margin_top_mm,
            page.margin_right_mm,
            page.margin_bottom_mm,
            page.margin_left_mm
        ));
    }
    output.push_str("</style>");
}

fn wrap_pdf_line(line: &str, limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    if line.trim().is_empty() {
        chunks.push(String::new());
        return chunks;
    }
    let mut current = String::new();
    for word in line.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > limit {
            if !current.is_empty() {
                chunks.push(current);
                current = String::new();
            }
            let mut word_chunk = String::new();
            for ch in word.chars() {
                if word_chunk.chars().count() >= limit {
                    chunks.push(word_chunk);
                    word_chunk = String::new();
                }
                word_chunk.push(ch);
            }
            if !word_chunk.is_empty() {
                current = word_chunk;
            }
        } else if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word_len <= limit {
            current.push(' ');
            current.push_str(word);
        } else {
            chunks.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn wrap_pdf_linked_line(line: &PdfLinkedText, limit: usize) -> Vec<PdfLinkedLine> {
    let limit = limit.max(1);
    let mut words: Vec<Vec<PdfLinkedChar>> = Vec::new();
    let mut current_word = Vec::new();
    let mut pending_destinations = line.pending_destinations.clone();

    for mut linked_char in line.to_chars() {
        if linked_char.ch.is_whitespace() {
            pending_destinations.extend(linked_char.destinations);
            if !current_word.is_empty() {
                words.push(current_word);
                current_word = Vec::new();
            }
        } else {
            if !pending_destinations.is_empty() {
                let mut destinations = std::mem::take(&mut pending_destinations);
                destinations.append(&mut linked_char.destinations);
                linked_char.destinations = destinations;
            }
            current_word.push(linked_char);
        }
    }
    if !current_word.is_empty() {
        words.push(current_word);
    }
    if words.is_empty() {
        return vec![PdfLinkedLine {
            text: String::new(),
            links: Vec::new(),
            destinations: pending_destinations
                .into_iter()
                .map(|id| PdfLineDestination { offset: 0, id })
                .collect(),
        }];
    }

    let mut lines = Vec::new();
    let mut current = Vec::new();
    for word in words {
        let word_len = word.len();
        if word_len > limit {
            if !current.is_empty() {
                lines.push(pdf_linked_line_from_chars(current));
                current = Vec::new();
            }

            let mut start = 0;
            while start < word.len() {
                let end = (start + limit).min(word.len());
                let chunk = word[start..end].to_vec();
                if end == word.len() {
                    current = chunk;
                } else {
                    lines.push(pdf_linked_line_from_chars(chunk));
                }
                start = end;
            }
        } else if current.is_empty() {
            current = word;
        } else if current.len() + 1 + word_len <= limit {
            let space_target = {
                let last_target = current
                    .last()
                    .and_then(|linked_char| linked_char.target.as_ref());
                let first_target = word
                    .first()
                    .and_then(|linked_char| linked_char.target.as_ref());
                if last_target.is_some() && last_target == first_target {
                    last_target.cloned()
                } else {
                    None
                }
            };
            current.push(PdfLinkedChar {
                ch: ' ',
                target: space_target,
                destinations: Vec::new(),
            });
            current.extend(word);
        } else {
            lines.push(pdf_linked_line_from_chars(current));
            current = word;
        }
    }
    if !current.is_empty() {
        lines.push(pdf_linked_line_from_chars(current));
    }
    if lines.is_empty() {
        lines.push(empty_pdf_linked_line());
    }
    lines
}

fn wrap_pdf_linked_text_lines(text: &PdfLinkedText, limit: usize) -> Vec<PdfLinkedLine> {
    let mut lines = Vec::new();
    for line in text.split_lines() {
        lines.extend(wrap_pdf_linked_line(&line, limit));
    }
    if lines.is_empty() {
        lines.push(empty_pdf_linked_line());
    }
    lines
}

fn empty_pdf_linked_line() -> PdfLinkedLine {
    PdfLinkedLine {
        text: String::new(),
        links: Vec::new(),
        destinations: Vec::new(),
    }
}

fn pdf_linked_line_from_chars(chars: Vec<PdfLinkedChar>) -> PdfLinkedLine {
    let mut text = String::new();
    let mut links = Vec::new();
    let mut destinations = Vec::new();
    let mut active_target: Option<PdfLinkTarget> = None;
    let mut active_start = 0;

    for (index, linked_char) in chars.iter().enumerate() {
        for id in &linked_char.destinations {
            destinations.push(PdfLineDestination {
                offset: index,
                id: id.clone(),
            });
        }
        if linked_char.target != active_target {
            if let Some(target) = active_target.take() {
                links.push(PdfLineLink {
                    start: active_start,
                    end: index,
                    target,
                });
            }
            if let Some(target) = linked_char.target.clone() {
                active_target = Some(target);
                active_start = index;
            }
        }
        text.push(linked_char.ch);
    }

    if let Some(target) = active_target {
        links.push(PdfLineLink {
            start: active_start,
            end: chars.len(),
            target,
        });
    }

    PdfLinkedLine {
        text,
        links,
        destinations,
    }
}

fn wrap_pdf_text_lines(text: &str, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        lines.extend(wrap_pdf_line(line, limit));
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn layout_pdf_table_row(row: &PdfProjectedTableRow, content_width: f32) -> PdfTableRowLayout {
    let cell_count = row.cells.len().max(1);
    let cell_widths = pdf_table_cell_widths(&row.column_widths, cell_count, content_width);
    let cells = if row.cells.is_empty() {
        vec![PdfTableCellLayout {
            lines: vec![empty_pdf_linked_line()],
            presentation: Default::default(),
        }]
    } else {
        row.cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let text_width = (cell_widths[index] - PDF_TABLE_CELL_PADDING * 2.0)
                    .max(PDF_TABLE_FONT_SIZE * 4.0);
                let max_chars = pdf_wrap_char_limit_for_width(text_width, PDF_TABLE_FONT_SIZE, 6);
                PdfTableCellLayout {
                    lines: wrap_pdf_linked_text_lines(&cell.text, max_chars),
                    presentation: cell.presentation.clone(),
                }
            })
            .collect()
    };
    let max_lines = cells.iter().map(|cell| cell.lines.len()).max().unwrap_or(1);
    let row_height = max_lines as f32 * PDF_TABLE_LEADING + PDF_TABLE_CELL_PADDING * 2.0;
    PdfTableRowLayout {
        cells,
        cell_widths,
        row_height,
        block_height: row_height + PDF_TABLE_ROW_GAP_POINTS,
    }
}

fn pdf_table_cell_widths(widths: &[u16], cell_count: usize, content_width: f32) -> Vec<f32> {
    let cell_count = cell_count.max(1);
    if let Some(sanitized) = sanitize_table_column_widths(widths, cell_count) {
        return sanitized
            .iter()
            .map(|width| {
                content_width * f32::from(*width)
                    / f32::from(word_core::TABLE_COLUMN_WIDTH_TOTAL_PER_MILLE)
            })
            .collect();
    }
    vec![content_width / cell_count as f32; cell_count]
}

fn layout_pdf_figure(figure: &PdfProjectedFigure, content_width: f32) -> PdfFigureLayout {
    let scale = figure.scale_percent.clamp(25, 200);
    let width_ratio = (scale as f32 / 100.0).min(1.0);
    let width = (content_width * width_ratio).clamp(content_width * 0.25, content_width);
    let text_width = (width - PDF_FIGURE_PADDING * 2.0).max(PDF_FIGURE_FONT_SIZE * 6.0);
    let max_chars = pdf_wrap_char_limit_for_width(text_width, PDF_FIGURE_FONT_SIZE, 8);
    let mut lines = Vec::new();
    if let Some(alt_text) = figure.alt_text.as_deref() {
        lines.extend(wrap_pdf_text_lines(alt_text, max_chars));
    }
    if let Some(caption) = figure.caption.as_deref() {
        lines.extend(wrap_pdf_text_lines(caption, max_chars));
    }

    let image = figure.jpeg.as_ref().map(|jpeg| {
        let image_width = (width - PDF_FIGURE_PADDING * 2.0).max(1.0);
        let image_height = (image_width * jpeg.height_px as f32 / jpeg.width_px as f32).max(1.0);
        PdfFigureImageLayout {
            asset_id: jpeg.asset_id.clone(),
            width_px: jpeg.width_px,
            height_px: jpeg.height_px,
            components: jpeg.components,
            width: image_width,
            height: image_height,
        }
    });

    if lines.is_empty() && image.is_none() {
        lines.push("Image".to_string());
    }
    let box_height = if let Some(image) = image.as_ref() {
        let text_height = if lines.is_empty() {
            0.0
        } else {
            PDF_FIGURE_IMAGE_TEXT_GAP_POINTS + lines.len() as f32 * PDF_FIGURE_LEADING
        };
        (PDF_FIGURE_PADDING * 2.0 + image.height + text_height).max(PDF_FIGURE_MIN_HEIGHT)
    } else {
        let scaled_height =
            (PDF_FIGURE_BASE_HEIGHT * (scale as f32 / 100.0)).max(PDF_FIGURE_MIN_HEIGHT);
        let text_height = lines.len() as f32 * PDF_FIGURE_LEADING + PDF_FIGURE_PADDING * 2.0;
        scaled_height.max(text_height)
    };
    PdfFigureLayout {
        lines,
        alignment: figure.alignment,
        width,
        box_height,
        block_height: box_height + PDF_FIGURE_GAP_POINTS,
        image,
    }
}

fn pdf_header_lines(
    document: &Document,
    section_index: usize,
    section_page_number: usize,
    field_page_number: usize,
    total_pages: usize,
) -> Vec<String> {
    let section = &document.sections[section_index];
    let region = if section.page_regions.different_first_page && section_page_number == 1 {
        &section.page_regions.first_header
    } else {
        &section.page_regions.header
    };
    pdf_region_lines(region, document, field_page_number, total_pages)
}

fn pdf_footer_lines(
    document: &Document,
    section_index: usize,
    section_page_number: usize,
    field_page_number: usize,
    total_pages: usize,
) -> Vec<String> {
    let section = &document.sections[section_index];
    let region = if section.page_regions.different_first_page && section_page_number == 1 {
        &section.page_regions.first_footer
    } else {
        &section.page_regions.footer
    };
    pdf_region_lines(region, document, field_page_number, total_pages)
}

fn pdf_region_lines(
    region: &PageRegion,
    document: &Document,
    page_number: usize,
    total_pages: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    for block in &region.blocks {
        match block {
            PageRegionBlock::Paragraph(paragraph) => {
                let mut line = String::new();
                push_inlines_pdf_text(&paragraph.inlines, document, &mut line);
                lines.push(resolve_pdf_page_fields(
                    &line,
                    page_number,
                    total_pages,
                    document,
                ));
            }
        }
    }
    lines
}

fn resolve_pdf_page_fields(
    text: &str,
    page_number: usize,
    total_pages: usize,
    document: &Document,
) -> String {
    text.replace(PDF_PAGE_NUMBER_TOKEN, &page_number.to_string())
        .replace(PDF_PAGE_COUNT_TOKEN, &total_pages.to_string())
        .replace(
            PDF_DATE_TOKEN,
            &document.meta.modified_at.format("%Y-%m-%d").to_string(),
        )
}

fn escape_pdf_text(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '(' => output.push_str("\\("),
            ')' => output.push_str("\\)"),
            '\t' => output.push_str("    "),
            ch if ch.is_ascii_graphic() || ch == ' ' => output.push(ch),
            _ => output.push('?'),
        }
    }
    output
}

fn build_pdf(document: &Document, pages: &[PdfPage], total_pages: usize) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let font_object_number = 3;
    let mut remaining_document_annotations = PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT;
    let mut remaining_document_images = PDF_MAX_EMBEDDED_IMAGES_PER_DOCUMENT;
    let destinations = collect_pdf_destinations(document, pages, total_pages);
    let mut rendered_pages = Vec::new();

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(Vec::new());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

    for page in pages {
        let page_annotation_limit =
            remaining_document_annotations.min(PDF_MAX_LINK_ANNOTATIONS_PER_PAGE);
        let rendered = render_pdf_page(
            document,
            page,
            total_pages,
            page_annotation_limit,
            remaining_document_images,
            &destinations,
        );
        remaining_document_annotations =
            remaining_document_annotations.saturating_sub(rendered.annotations.len());
        remaining_document_images = remaining_document_images.saturating_sub(rendered.images.len());
        rendered_pages.push(rendered);
    }

    let page_object_numbers = pdf_page_object_numbers(pages, &rendered_pages);
    let mut kids = Vec::new();

    for (page, rendered) in pages.iter().zip(rendered_pages) {
        let page_object_number = *page_object_numbers
            .get(&page.page_number)
            .expect("selected PDF page should have an object number");
        let content_object_number = page_object_number + 1;
        let first_annotation_object_number = content_object_number + 1;
        let first_image_object_number = first_annotation_object_number + rendered.annotations.len();
        let annotation_refs = rendered
            .annotations
            .iter()
            .enumerate()
            .map(|(index, _)| format!("{} 0 R", first_annotation_object_number + index))
            .collect::<Vec<_>>();
        let annotations_entry = if annotation_refs.is_empty() {
            String::new()
        } else {
            format!(" /Annots [{}]", annotation_refs.join(" "))
        };
        let xobject_resources = if rendered.images.is_empty() {
            String::new()
        } else {
            let image_refs = rendered
                .images
                .iter()
                .enumerate()
                .map(|(index, image)| {
                    format!("/{} {} 0 R", image.name, first_image_object_number + index)
                })
                .collect::<Vec<_>>();
            format!(" /XObject << {} >>", image_refs.join(" "))
        };
        kids.push(format!("{page_object_number} 0 R"));
        let page_width = mm_to_points(page.page_setup.width_mm);
        let page_height = mm_to_points(page.page_setup.height_mm);
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width:.1} {page_height:.1}] /Resources << /Font << /F1 {font_object_number} 0 R >>{xobject_resources} >> /Contents {content_object_number} 0 R{annotations_entry} >>"
            )
            .into_bytes(),
        );
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                rendered.stream.len(),
                rendered.stream
            )
            .into_bytes(),
        );
        for annotation in rendered.annotations {
            objects.push(
                pdf_link_annotation_object(&annotation, &destinations, &page_object_numbers)
                    .into_bytes(),
            );
        }
        for image in rendered.images {
            objects.push(pdf_image_xobject_object(&image));
        }
    }

    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    )
    .into_bytes();

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0);
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_start = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn pdf_page_object_numbers(
    pages: &[PdfPage],
    rendered_pages: &[PdfRenderedPage],
) -> BTreeMap<usize, usize> {
    let mut numbers = BTreeMap::new();
    let mut next_object_number = 4;
    for (page, rendered) in pages.iter().zip(rendered_pages) {
        numbers.insert(page.page_number, next_object_number);
        next_object_number += 2 + rendered.annotations.len() + rendered.images.len();
    }
    numbers
}

fn collect_pdf_destinations(
    document: &Document,
    pages: &[PdfPage],
    total_pages: usize,
) -> BTreeMap<PdfDestinationId, PdfDestination> {
    let mut candidates: BTreeMap<PdfDestinationId, Option<PdfDestination>> = BTreeMap::new();
    for page in pages {
        let page_width = mm_to_points(page.page_setup.width_mm);
        if page_width <= 0.0 {
            continue;
        }
        let margin_left = mm_to_points(page.page_setup.margin_left_mm).max(PDF_MIN_MARGIN_POINTS);
        let margin_right = mm_to_points(page.page_setup.margin_right_mm).max(PDF_MIN_MARGIN_POINTS);
        let content_width = (page_width - margin_left - margin_right).max(PDF_FONT_SIZE * 20.0);
        let (body_start_y, body_bottom_y) = pdf_body_vertical_bounds(
            document,
            page.section_index,
            page.section_page_number,
            page.page_number,
            total_pages,
        );
        let mut cursor_y = body_start_y.max(body_bottom_y + PDF_LEADING);
        let context = PdfDestinationCollectionContext {
            document,
            page,
            total_pages,
            margin_left,
            content_width,
        };
        for item in &page.body_items {
            collect_pdf_body_item_destinations(&context, item, &mut cursor_y, &mut candidates);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(id, destination)| destination.map(|destination| (id, destination)))
        .collect()
}

fn collect_pdf_body_item_destinations(
    context: &PdfDestinationCollectionContext<'_>,
    item: &PdfPageBodyItem,
    cursor_y: &mut f32,
    candidates: &mut BTreeMap<PdfDestinationId, Option<PdfDestination>>,
) {
    match item {
        PdfPageBodyItem::TextLine(line) => {
            let resolved = resolve_pdf_linked_line(
                line,
                context.page.page_number,
                context.total_pages,
                context.document,
            );
            collect_pdf_line_destinations(
                &resolved,
                PDF_FONT_SIZE,
                context.margin_left,
                *cursor_y,
                context.page.page_number,
                candidates,
            );
            *cursor_y -= PDF_LEADING;
        }
        PdfPageBodyItem::TableRow(row) => {
            let top_y = *cursor_y + PDF_TABLE_CELL_PADDING;
            let mut x = context.margin_left;
            for (cell_index, cell) in row.cells.iter().enumerate() {
                let cell_width = row
                    .cell_widths
                    .get(cell_index)
                    .copied()
                    .unwrap_or(context.content_width / row.cells.len().max(1) as f32);
                let lines = resolve_pdf_linked_lines(
                    &cell.lines,
                    context.page.page_number,
                    context.total_pages,
                    context.document,
                );
                for (line_index, line) in lines.iter().enumerate() {
                    let line_x = pdf_table_cell_line_x(
                        x,
                        cell_width,
                        line,
                        cell.presentation.text_alignment,
                    );
                    let y = top_y
                        - PDF_TABLE_CELL_PADDING
                        - PDF_TABLE_FONT_SIZE
                        - line_index as f32 * PDF_TABLE_LEADING;
                    collect_pdf_line_destinations(
                        line,
                        PDF_TABLE_FONT_SIZE,
                        line_x,
                        y,
                        context.page.page_number,
                        candidates,
                    );
                }
                x += cell_width;
            }
            *cursor_y -= row.block_height;
        }
        PdfPageBodyItem::Figure(figure) => {
            *cursor_y -= figure.block_height;
        }
    }
}

fn collect_pdf_line_destinations(
    line: &PdfLinkedLine,
    font_size: f32,
    x: f32,
    baseline_y: f32,
    page_number: usize,
    candidates: &mut BTreeMap<PdfDestinationId, Option<PdfDestination>>,
) {
    let char_width = font_size * PDF_TEXT_WIDTH_FACTOR;
    for destination in &line.destinations {
        let left = x + destination.offset as f32 * char_width;
        push_pdf_destination_candidate(
            candidates,
            destination.id.clone(),
            PdfDestination {
                page_number,
                left,
                top: baseline_y + font_size,
            },
        );
    }
}

fn push_pdf_destination_candidate(
    candidates: &mut BTreeMap<PdfDestinationId, Option<PdfDestination>>,
    id: PdfDestinationId,
    destination: PdfDestination,
) {
    match candidates.get_mut(&id) {
        Some(existing) => *existing = None,
        None => {
            candidates.insert(id, Some(destination));
        }
    }
}

fn render_pdf_page(
    document: &Document,
    page: &PdfPage,
    total_pages: usize,
    annotation_limit: usize,
    image_limit: usize,
    destinations: &BTreeMap<PdfDestinationId, PdfDestination>,
) -> PdfRenderedPage {
    let page_width = mm_to_points(page.page_setup.width_mm);
    let page_height = mm_to_points(page.page_setup.height_mm);
    let margin_left = mm_to_points(page.page_setup.margin_left_mm).max(PDF_MIN_MARGIN_POINTS);
    let margin_top = mm_to_points(page.page_setup.margin_top_mm).max(PDF_MIN_MARGIN_POINTS);
    let margin_bottom = mm_to_points(page.page_setup.margin_bottom_mm).max(PDF_MIN_MARGIN_POINTS);
    let margin_right = mm_to_points(page.page_setup.margin_right_mm).max(PDF_MIN_MARGIN_POINTS);
    let content_width = (page_width - margin_left - margin_right).max(PDF_FONT_SIZE * 20.0);
    let header_lines = pdf_header_lines(
        document,
        page.section_index,
        page.section_page_number,
        page.page_number,
        total_pages,
    );
    let footer_lines = pdf_footer_lines(
        document,
        page.section_index,
        page.section_page_number,
        page.page_number,
        total_pages,
    );
    let footer_height = pdf_region_height(footer_lines.len());
    let (body_start_y, body_bottom_y) = pdf_body_vertical_bounds(
        document,
        page.section_index,
        page.section_page_number,
        page.page_number,
        total_pages,
    );
    let footer_start_y =
        (margin_bottom + footer_height - PDF_REGION_LEADING).max(PDF_MIN_MARGIN_POINTS / 2.0);

    let mut stream = String::new();
    let mut annotations = Vec::new();
    let mut images = Vec::new();
    let mut remaining_annotations = annotation_limit;
    let mut remaining_images = image_limit;
    if !header_lines.is_empty() {
        push_pdf_text_block(
            &mut stream,
            PDF_REGION_FONT_SIZE,
            PDF_REGION_LEADING,
            margin_left,
            page_height - margin_top,
            &header_lines,
        );
    }

    let mut cursor_y = body_start_y.max(body_bottom_y + PDF_LEADING);
    let render_context = PdfPageRenderContext {
        document,
        page,
        total_pages,
        destinations,
        margin_left,
        content_width,
    };
    for item in &page.body_items {
        let mut render_state = PdfPageRenderState {
            cursor_y: &mut cursor_y,
            annotations: &mut annotations,
            remaining_annotations: &mut remaining_annotations,
            images: &mut images,
            remaining_images: &mut remaining_images,
        };
        push_pdf_body_item_stream(&mut stream, &render_context, item, &mut render_state);
    }

    if !footer_lines.is_empty() {
        push_pdf_text_block(
            &mut stream,
            PDF_REGION_FONT_SIZE,
            PDF_REGION_LEADING,
            margin_left,
            footer_start_y,
            &footer_lines,
        );
    }

    if page_width > 0.0 {
        PdfRenderedPage {
            stream,
            annotations,
            images,
        }
    } else {
        PdfRenderedPage {
            stream: String::new(),
            annotations: Vec::new(),
            images: Vec::new(),
        }
    }
}

fn push_pdf_body_item_stream(
    stream: &mut String,
    context: &PdfPageRenderContext<'_>,
    item: &PdfPageBodyItem,
    state: &mut PdfPageRenderState<'_>,
) {
    match item {
        PdfPageBodyItem::TextLine(line) => {
            let resolved = resolve_pdf_linked_line(
                line,
                context.page.page_number,
                context.total_pages,
                context.document,
            );
            push_pdf_linked_text_block(
                stream,
                PDF_FONT_SIZE,
                PDF_LEADING,
                context.margin_left,
                *state.cursor_y,
                &[resolved],
                &mut PdfAnnotationCollector {
                    annotations: state.annotations,
                    remaining: state.remaining_annotations,
                    destinations: context.destinations,
                },
            );
            *state.cursor_y -= PDF_LEADING;
        }
        PdfPageBodyItem::TableRow(row) => {
            let top_y = *state.cursor_y + PDF_TABLE_CELL_PADDING;
            let bottom_y = top_y - row.row_height;
            let mut x = context.margin_left;
            for (cell_index, cell) in row.cells.iter().enumerate() {
                let cell_width = row
                    .cell_widths
                    .get(cell_index)
                    .copied()
                    .unwrap_or(context.content_width / row.cells.len().max(1) as f32);
                if let Some(background_color) = cell
                    .presentation
                    .background_color
                    .as_deref()
                    .and_then(sanitize_table_cell_background_color)
                {
                    let (red, green, blue) = pdf_table_cell_background_rgb(&background_color);
                    stream.push_str(&format!(
                        "q {red:.3} {green:.3} {blue:.3} rg {x:.1} {bottom_y:.1} {cell_width:.1} {:.1} re f Q\n",
                        row.row_height
                    ));
                }
                x += cell_width;
            }
            stream.push_str("q 0.6 w 0.45 G\n");
            let mut x = context.margin_left;
            for (cell_index, cell) in row.cells.iter().enumerate() {
                let cell_width = row
                    .cell_widths
                    .get(cell_index)
                    .copied()
                    .unwrap_or(context.content_width / row.cells.len().max(1) as f32);
                if cell.presentation.border == TableCellBorder::Hidden {
                    x += cell_width;
                    continue;
                }
                stream.push_str(&format!(
                    "{x:.1} {bottom_y:.1} {cell_width:.1} {:.1} re S\n",
                    row.row_height
                ));
                x += cell_width;
            }
            stream.push_str("Q\n");
            let mut x = context.margin_left;
            for (cell_index, cell) in row.cells.iter().enumerate() {
                let cell_width = row
                    .cell_widths
                    .get(cell_index)
                    .copied()
                    .unwrap_or(context.content_width / row.cells.len().max(1) as f32);
                let lines = resolve_pdf_linked_lines(
                    &cell.lines,
                    context.page.page_number,
                    context.total_pages,
                    context.document,
                );
                for (line_index, line) in lines.iter().enumerate() {
                    let x = pdf_table_cell_line_x(
                        x,
                        cell_width,
                        line,
                        cell.presentation.text_alignment,
                    );
                    let y = top_y
                        - PDF_TABLE_CELL_PADDING
                        - PDF_TABLE_FONT_SIZE
                        - line_index as f32 * PDF_TABLE_LEADING;
                    push_pdf_linked_text_block(
                        stream,
                        PDF_TABLE_FONT_SIZE,
                        PDF_TABLE_LEADING,
                        x,
                        y,
                        std::slice::from_ref(line),
                        &mut PdfAnnotationCollector {
                            annotations: state.annotations,
                            remaining: state.remaining_annotations,
                            destinations: context.destinations,
                        },
                    );
                }
                x += cell_width;
            }
            *state.cursor_y -= row.block_height;
        }
        PdfPageBodyItem::Figure(figure) => {
            let top_y = *state.cursor_y + PDF_FIGURE_PADDING / 2.0;
            let bottom_y = top_y - figure.box_height;
            let x = pdf_aligned_x(
                context.margin_left,
                context.content_width,
                figure.width,
                figure.alignment,
            );
            stream.push_str(&format!(
                "q 0.7 w 0.35 G {x:.1} {bottom_y:.1} {:.1} {:.1} re S Q\n",
                figure.width, figure.box_height
            ));
            if let Some(image) = figure.image.as_ref() {
                if let Some(image_name) = register_pdf_image(
                    image,
                    &mut PdfImageCollector {
                        document: context.document,
                        images: state.images,
                        remaining: state.remaining_images,
                    },
                ) {
                    let inner_width = (figure.width - PDF_FIGURE_PADDING * 2.0).max(1.0);
                    let image_x =
                        x + PDF_FIGURE_PADDING + (inner_width - image.width).max(0.0) / 2.0;
                    let image_y = top_y - PDF_FIGURE_PADDING - image.height;
                    stream.push_str(&format!(
                        "q {:.1} 0 0 {:.1} {image_x:.1} {image_y:.1} cm /{image_name} Do Q\n",
                        image.width, image.height
                    ));
                    let y = image_y - PDF_FIGURE_IMAGE_TEXT_GAP_POINTS - PDF_FIGURE_FONT_SIZE;
                    push_pdf_figure_text(stream, context, figure, x, y);
                } else {
                    let y = top_y - PDF_FIGURE_PADDING - PDF_FIGURE_FONT_SIZE;
                    push_pdf_figure_placeholder_text(stream, context, figure, x, y);
                }
            } else {
                let y = top_y - PDF_FIGURE_PADDING - PDF_FIGURE_FONT_SIZE;
                push_pdf_figure_placeholder_text(stream, context, figure, x, y);
            }
            *state.cursor_y -= figure.block_height;
        }
    }
}

fn push_pdf_figure_text(
    stream: &mut String,
    context: &PdfPageRenderContext<'_>,
    figure: &PdfFigureLayout,
    x: f32,
    y: f32,
) {
    if figure.lines.is_empty() {
        return;
    }
    let lines = resolve_pdf_text_lines(
        &figure.lines,
        context.page.page_number,
        context.total_pages,
        context.document,
    );
    push_pdf_text_block(
        stream,
        PDF_FIGURE_FONT_SIZE,
        PDF_FIGURE_LEADING,
        x + PDF_FIGURE_PADDING,
        y,
        &lines,
    );
}

fn push_pdf_figure_placeholder_text(
    stream: &mut String,
    context: &PdfPageRenderContext<'_>,
    figure: &PdfFigureLayout,
    x: f32,
    y: f32,
) {
    let fallback_lines;
    let lines = if figure.lines.is_empty() {
        fallback_lines = vec!["Image".to_string()];
        &fallback_lines
    } else {
        &figure.lines
    };
    let resolved = resolve_pdf_text_lines(
        lines,
        context.page.page_number,
        context.total_pages,
        context.document,
    );
    push_pdf_text_block(
        stream,
        PDF_FIGURE_FONT_SIZE,
        PDF_FIGURE_LEADING,
        x + PDF_FIGURE_PADDING,
        y,
        &resolved,
    );
}

fn register_pdf_image(
    image: &PdfFigureImageLayout,
    collector: &mut PdfImageCollector<'_>,
) -> Option<String> {
    if *collector.remaining == 0 {
        return None;
    }
    let asset = collector.document.assets.get(&image.asset_id)?;
    let safe = pdf_safe_jpeg_asset(asset)?;
    if safe.info.width_px != image.width_px
        || safe.info.height_px != image.height_px
        || safe.info.components != image.components
    {
        return None;
    }

    let name = format!("Im{}", collector.images.len() + 1);
    collector.images.push(PdfRenderedImage {
        name: name.clone(),
        bytes: safe.bytes,
        width_px: safe.info.width_px,
        height_px: safe.info.height_px,
        components: safe.info.components,
    });
    *collector.remaining -= 1;
    Some(name)
}

fn resolve_pdf_text_lines(
    lines: &[String],
    page_number: usize,
    total_pages: usize,
    document: &Document,
) -> Vec<String> {
    lines
        .iter()
        .map(|line| resolve_pdf_page_fields(line, page_number, total_pages, document))
        .collect()
}

fn resolve_pdf_linked_lines(
    lines: &[PdfLinkedLine],
    page_number: usize,
    total_pages: usize,
    document: &Document,
) -> Vec<PdfLinkedLine> {
    lines
        .iter()
        .map(|line| resolve_pdf_linked_line(line, page_number, total_pages, document))
        .collect()
}

fn resolve_pdf_linked_line(
    line: &PdfLinkedLine,
    page_number: usize,
    total_pages: usize,
    document: &Document,
) -> PdfLinkedLine {
    let resolved = resolve_pdf_page_fields(&line.text, page_number, total_pages, document);
    let links = if resolved == line.text {
        line.links.clone()
    } else {
        Vec::new()
    };
    PdfLinkedLine {
        text: resolved,
        links,
        destinations: line.destinations.clone(),
    }
}

fn pdf_aligned_x(
    margin_left: f32,
    content_width: f32,
    item_width: f32,
    alignment: ImageAlignment,
) -> f32 {
    match alignment {
        ImageAlignment::Center => margin_left + (content_width - item_width).max(0.0) / 2.0,
        ImageAlignment::Right => margin_left + (content_width - item_width).max(0.0),
        ImageAlignment::Inline | ImageAlignment::Left => margin_left,
    }
}

fn pdf_table_cell_line_x(
    cell_left: f32,
    cell_width: f32,
    line: &PdfLinkedLine,
    alignment: Option<ParagraphAlignment>,
) -> f32 {
    let content_width = (cell_width - PDF_TABLE_CELL_PADDING * 2.0).max(0.0);
    let text_width = approximate_pdf_text_width(&line.text, PDF_TABLE_FONT_SIZE).min(content_width);
    let offset = match alignment {
        Some(ParagraphAlignment::Center) => (content_width - text_width).max(0.0) / 2.0,
        Some(ParagraphAlignment::Right) => (content_width - text_width).max(0.0),
        Some(ParagraphAlignment::Left | ParagraphAlignment::Justify) | None => 0.0,
    };
    cell_left + PDF_TABLE_CELL_PADDING + offset
}

fn approximate_pdf_text_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * PDF_TEXT_WIDTH_FACTOR
}

fn pdf_table_cell_background_rgb(background_color: &str) -> (f32, f32, f32) {
    match background_color {
        "#f1f5f9" => (0.945, 0.961, 0.976),
        "#fff3bf" => (1.0, 0.953, 0.749),
        "#dbeafe" => (0.859, 0.918, 0.996),
        "#dcfce7" => (0.863, 0.988, 0.906),
        _ => (1.0, 1.0, 1.0),
    }
}

fn push_pdf_text_block(
    stream: &mut String,
    font_size: f32,
    leading: f32,
    x: f32,
    y: f32,
    lines: &[String],
) {
    stream.push_str(&format!(
        "BT /F1 {font_size:.1} Tf {x:.1} {y:.1} Td {leading:.1} TL\n"
    ));
    for line in lines {
        stream.push('(');
        stream.push_str(&escape_pdf_text(line));
        stream.push_str(") Tj T*\n");
    }
    stream.push_str("ET\n");
}

fn push_pdf_linked_text_block(
    stream: &mut String,
    font_size: f32,
    leading: f32,
    x: f32,
    y: f32,
    lines: &[PdfLinkedLine],
    annotation_collector: &mut PdfAnnotationCollector<'_>,
) {
    stream.push_str(&format!(
        "BT /F1 {font_size:.1} Tf {x:.1} {y:.1} Td {leading:.1} TL\n"
    ));
    for (line_index, line) in lines.iter().enumerate() {
        let baseline_y = y - line_index as f32 * leading;
        push_pdf_line_annotations(
            line,
            font_size,
            x,
            baseline_y,
            annotation_collector.annotations,
            annotation_collector.remaining,
            annotation_collector.destinations,
        );
        stream.push('(');
        stream.push_str(&escape_pdf_text(&line.text));
        stream.push_str(") Tj T*\n");
    }
    stream.push_str("ET\n");
}

fn push_pdf_line_annotations(
    line: &PdfLinkedLine,
    font_size: f32,
    x: f32,
    baseline_y: f32,
    annotations: &mut Vec<PdfLinkAnnotation>,
    remaining_annotations: &mut usize,
    destinations: &BTreeMap<PdfDestinationId, PdfDestination>,
) {
    if *remaining_annotations == 0 {
        return;
    }
    let char_width = font_size * PDF_TEXT_WIDTH_FACTOR;
    for link in &line.links {
        if *remaining_annotations == 0 {
            break;
        }
        if link.start >= link.end {
            continue;
        }
        let left = x + link.start as f32 * char_width;
        let right = (x + link.end as f32 * char_width).max(left + font_size * 0.5);
        let target = match &link.target {
            PdfLinkTarget::Uri(uri) => PdfAnnotationTarget::Uri(uri.clone()),
            PdfLinkTarget::Destination(destination_id) => {
                if !destinations.contains_key(destination_id) {
                    continue;
                }
                PdfAnnotationTarget::Destination(destination_id.clone())
            }
        };
        annotations.push(PdfLinkAnnotation {
            rect: PdfRect {
                left,
                bottom: baseline_y - font_size * 0.25,
                right,
                top: baseline_y + font_size,
            },
            target,
        });
        *remaining_annotations -= 1;
    }
}

fn pdf_link_annotation_object(
    annotation: &PdfLinkAnnotation,
    destinations: &BTreeMap<PdfDestinationId, PdfDestination>,
    page_object_numbers: &BTreeMap<usize, usize>,
) -> String {
    let action = match &annotation.target {
        PdfAnnotationTarget::Uri(uri) => {
            format!("/A << /S /URI /URI ({}) >>", escape_pdf_text(uri))
        }
        PdfAnnotationTarget::Destination(destination_id) => {
            let destination = destinations
                .get(destination_id)
                .expect("internal PDF annotation should reference a collected destination");
            let page_object_number = page_object_numbers
                .get(&destination.page_number)
                .expect("internal PDF destination should reference a selected page object");
            format!(
                "/Dest [{page_object_number} 0 R /XYZ {:.1} {:.1} 0]",
                destination.left, destination.top
            )
        }
    };
    format!(
        "<< /Type /Annot /Subtype /Link /Rect [{:.1} {:.1} {:.1} {:.1}] /Border [0 0 0] {action} >>",
        annotation.rect.left,
        annotation.rect.bottom,
        annotation.rect.right,
        annotation.rect.top,
    )
}

fn pdf_image_xobject_object(image: &PdfRenderedImage) -> Vec<u8> {
    let color_space = match image.components {
        1 => "/DeviceGray",
        3 => "/DeviceRGB",
        _ => "/DeviceRGB",
    };
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {color_space} /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        image.width_px,
        image.height_px,
        image.bytes.len()
    )
    .into_bytes();
    object.extend_from_slice(&image.bytes);
    object.extend_from_slice(b"\nendstream");
    object
}

fn mm_to_points(value: u16) -> f32 {
    value as f32 * POINTS_PER_MM
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use word_core::{
        ImageAlignment, ImageBlock, ImagePresentation, InlineNoteReference, ListBlock,
        ListDefinition, ListItem, Note, NoteKind, PageRegionParagraph, Table, TableCell,
        TableOfContents, TableOfContentsEntry, TableRow,
    };

    #[test]
    fn txt_export_preserves_empty_document_text() {
        let document = Document::new_untitled();

        assert_eq!(
            export_txt(&document).expect("txt export should succeed"),
            ""
        );
    }

    #[test]
    fn txt_export_covers_lists_tables_and_page_breaks() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::List(ListBlock {
                definition_id: "unordered".to_string(),
                items: vec![ListItem {
                    level: 0,
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: "body".into(),
                        format: Default::default(),
                        inlines: vec![Inline::text("List item")],
                    })],
                }],
            }),
            Block::Table(Table {
                column_widths: Vec::new(),
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        presentation: Default::default(),
                        blocks: vec![Block::Paragraph(Paragraph {
                            bookmark_id: None,
                            style: "body".into(),
                            format: Default::default(),
                            inlines: vec![Inline::text("Cell text")],
                        })],
                    }],
                }],
            }),
            Block::PageBreak,
        ];

        let text = export_txt(&document).expect("txt export should succeed");

        assert!(text.contains("List item"));
        assert!(text.contains("Cell text"));
        assert!(text.contains("--- page break ---"));
    }

    #[test]
    fn exports_include_page_regions_and_render_fields() {
        let mut document = Document::new_untitled();
        document.sections[0].page_regions.header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![
                    Inline::text("Header "),
                    Inline::field(PageField::PageNumber),
                    Inline::text("/"),
                    Inline::field(PageField::PageCount),
                ],
            })];
        document.sections[0].page_regions.footer.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Footer "), Inline::field(PageField::Date)],
            })];

        let text = export_txt(&document).expect("txt export should succeed");
        let html = export_html(&document).expect("html export should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(text.contains("[Header]\nHeader 1/1"));
        assert!(text.contains("[Footer]\nFooter "));
        assert!(html.contains("data-page-region=\"header\""));
        assert!(html.contains("data-page-field=\"page-number\">1</span>"));
        assert!(html.contains("data-page-field=\"date\""));
        assert!(pdf
            .windows("Header 1/1".len())
            .any(|window| window == b"Header 1/1"));
    }

    #[test]
    fn html_export_escapes_text() {
        let mut document = Document::new_untitled();
        document.meta.title = "<Draft>".to_string();

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("&lt;Draft&gt;"));
    }

    #[test]
    fn html_export_strips_unsafe_links_and_preserves_marks() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![Inline {
                text: "unsafe".to_string(),
                marks: vec![InlineMark::Bold],
                link: Some("javascript:alert(1)".to_string()),
                comment_ids: Vec::new(),
                style: Default::default(),
                field: None,
                note_reference: None,
                tracked_change: None,
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<strong>unsafe</strong>"));
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn html_export_preserves_safe_bookmarks_and_internal_links() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Heading(Heading {
                bookmark_id: Some("bm-heading".to_string()),
                level: 2,
                inlines: vec![Inline::text("Target")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: Some("bm-body".to_string()),
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline {
                    text: "Jump".to_string(),
                    marks: Vec::new(),
                    link: Some("#bm-heading".to_string()),
                    comment_ids: Vec::new(),
                    style: Default::default(),
                    field: None,
                    note_reference: None,
                    tracked_change: None,
                }],
            }),
        ];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<h2 id=\"bm-heading\">Target</h2>"));
        assert!(html.contains("<p id=\"bm-body\" data-style=\"body\">"));
        assert!(html.contains("href=\"#bm-heading\""));
        assert!(!html.contains("#../bad"));
    }

    #[test]
    fn exports_table_of_contents_as_text_and_safe_html_links_without_page_claims() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::TableOfContents(TableOfContents {
                title: "Contents".to_string(),
                entries: vec![
                    TableOfContentsEntry {
                        level: 1,
                        text: "Intro".to_string(),
                        target_bookmark_id: "bm-intro".to_string(),
                    },
                    TableOfContentsEntry {
                        level: 3,
                        text: "Details".to_string(),
                        target_bookmark_id: "bm-details".to_string(),
                    },
                    TableOfContentsEntry {
                        level: 2,
                        text: "Unsafe skipped".to_string(),
                        target_bookmark_id: "../bad".to_string(),
                    },
                ],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-intro".to_string()),
                level: 1,
                inlines: vec![Inline::text("Intro")],
            }),
        ];

        let text = export_txt(&document).expect("txt export should succeed");
        let html = export_html(&document).expect("html export should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(text.contains("Contents\nIntro\n    Details\n  Unsafe skipped"));
        assert!(html.contains("data-900word-block=\"table-of-contents\""));
        assert!(html.contains("href=\"#bm-intro\""));
        assert!(html.contains("data-toc-level=\"3\""));
        assert!(!html.contains("../bad"));
        assert!(!html.contains("data-page-field"));
        assert!(pdf
            .windows("Contents".len())
            .any(|window| window == b"Contents"));
    }

    #[test]
    fn note_exports_include_references_and_bodies_without_page_layout_claims() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.notes.insert(
            "note-end".to_string(),
            Note {
                id: "note-end".to_string(),
                kind: NoteKind::Endnote,
                body: "Endnote body".to_string(),
            },
        );
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![
                Inline::text("Claim"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Footnote,
                    label: "1".to_string(),
                }),
                Inline::text(" appendix"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-end".to_string(),
                    kind: NoteKind::Endnote,
                    label: "i".to_string(),
                }),
            ],
        })];

        let text = export_txt(&document).expect("txt export should succeed");
        let html = export_html(&document).expect("html export should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(text.contains("Claim1 appendixi"));
        assert!(text.contains("Footnotes\n[1] Source body"));
        assert!(text.contains("Endnotes\n[i] Endnote body"));
        assert!(html.contains("data-note-reference-id=\"note-source\""));
        assert!(html.contains("data-900word-notes=\"true\""));
        assert!(html.contains("Source body"));
        assert!(!html.contains("page-bottom"));
        assert!(pdf
            .windows("Footnotes".len())
            .any(|window| window == b"Footnotes"));
        let pdf_text = String::from_utf8_lossy(&pdf);
        assert_eq!(pdf_link_annotation_count(&pdf_text), 4);
        assert_eq!(pdf_dest_annotation_count(&pdf_text), 4);
        assert!(!pdf_text.contains("/S /URI"));
        assert!(!pdf_text.contains("note-source"));
        assert!(!pdf_text.contains("note-end"));
        assert!(pdf_contains(&pdf, "Source body"));
        assert!(pdf_contains(&pdf, "Endnote body"));
    }

    #[test]
    fn pdf_export_omits_note_links_when_note_body_is_outside_selected_page_range() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![
                    Inline::text("Claim"),
                    Inline::note_reference(InlineNoteReference {
                        id: "note-source".to_string(),
                        kind: NoteKind::Footnote,
                        label: "1".to_string(),
                    }),
                ],
            }),
            Block::PageBreak,
        ];

        let pdf = export_pdf_with_options(
            &document,
            PdfExportOptions {
                page_range: Some(PdfPageRange { start: 1, end: 1 }),
            },
        )
        .expect("valid page range should export");
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&pdf_text), 1);
        assert_eq!(pdf_link_annotation_count(&pdf_text), 0);
        assert_eq!(pdf_dest_annotation_count(&pdf_text), 0);
        assert!(!pdf_text.contains("/Annots ["));
        assert!(!pdf_text.contains("note-source"));
        assert!(pdf_contains(&pdf, "Claim"));
        assert!(!pdf_contains(&pdf, "Footnotes"));
        assert!(!pdf_contains(&pdf, "Source body"));
    }

    #[test]
    fn pdf_export_omits_note_backlinks_when_reference_is_outside_selected_page_range() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![
                    Inline::text("Claim"),
                    Inline::note_reference(InlineNoteReference {
                        id: "note-source".to_string(),
                        kind: NoteKind::Footnote,
                        label: "1".to_string(),
                    }),
                ],
            }),
            Block::PageBreak,
        ];

        let pdf = export_pdf_with_options(
            &document,
            PdfExportOptions {
                page_range: Some(PdfPageRange { start: 2, end: 2 }),
            },
        )
        .expect("valid page range should export");
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&pdf_text), 1);
        assert_eq!(pdf_link_annotation_count(&pdf_text), 0);
        assert_eq!(pdf_dest_annotation_count(&pdf_text), 0);
        assert!(!pdf_text.contains("/Annots ["));
        assert!(!pdf_text.contains("note-source"));
        assert!(!pdf_contains(&pdf, "Claim"));
        assert!(pdf_contains(&pdf, "Footnotes"));
        assert!(pdf_contains(&pdf, "Source body"));
    }

    #[test]
    fn pdf_export_keeps_mismatched_duplicate_note_reference_plain() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![
                Inline::text("Claim"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Footnote,
                    label: "1".to_string(),
                }),
                Inline::text(" later"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Endnote,
                    label: "i".to_string(),
                }),
            ],
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&pdf_text), 2);
        assert_eq!(pdf_dest_annotation_count(&pdf_text), 2);
        assert!(!pdf_text.contains("note-source"));
        assert!(pdf_contains(&pdf, "Claim1 lateri"));
        assert!(pdf_contains(&pdf, "Footnotes"));
        assert!(pdf_contains(&pdf, "Source body"));
    }

    #[test]
    fn pdf_export_links_later_valid_note_reference_after_mismatched_duplicate() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![
                Inline::text("Bad"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Endnote,
                    label: "i".to_string(),
                }),
                Inline::text(" good"),
                Inline::note_reference(InlineNoteReference {
                    id: "note-source".to_string(),
                    kind: NoteKind::Footnote,
                    label: "1".to_string(),
                }),
            ],
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&pdf_text), 2);
        assert_eq!(pdf_dest_annotation_count(&pdf_text), 2);
        assert!(!pdf_text.contains("note-source"));
        assert!(pdf_contains(&pdf, "Badi good1"));
        assert!(pdf_contains(&pdf, "Footnotes"));
        assert!(pdf_contains(&pdf, "Source body"));
    }

    #[test]
    fn pdf_export_note_destinations_do_not_collide_with_matching_bookmark_ids() {
        let mut document = Document::new_untitled();
        document.notes.insert(
            "note-source".to_string(),
            Note {
                id: "note-source".to_string(),
                kind: NoteKind::Footnote,
                body: "Source body".to_string(),
            },
        );
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![
                    linked_inline("Jump bookmark", "#note-body-note-source"),
                    Inline::text(" Claim"),
                    Inline::note_reference(InlineNoteReference {
                        id: "note-source".to_string(),
                        kind: NoteKind::Footnote,
                        label: "1".to_string(),
                    }),
                ],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("note-body-note-source".to_string()),
                level: 2,
                inlines: vec![Inline::text("Bookmark target")],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&pdf_text), 3);
        assert_eq!(pdf_dest_annotation_count(&pdf_text), 3);
        assert!(!pdf_text.contains("note-source"));
        assert!(pdf_contains(&pdf, "Jump bookmark"));
        assert!(pdf_contains(&pdf, "Claim1"));
        assert!(pdf_contains(&pdf, "Bookmark target"));
        assert!(pdf_contains(&pdf, "Source body"));
    }

    #[test]
    fn html_export_preserves_authoring_direct_formatting() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "quote".into(),
            format: word_core::ParagraphFormat {
                alignment: Some(word_core::ParagraphAlignment::Center),
                line_spacing_per_mille: Some(1500),
                spacing_before_mm: None,
                spacing_after_mm: Some(4),
                indent_start_mm: None,
                indent_end_mm: None,
                first_line_indent_mm: None,
            },
            inlines: vec![Inline {
                text: "Styled".to_string(),
                marks: vec![],
                link: None,
                comment_ids: Vec::new(),
                style: word_core::InlineStyle {
                    font_family: Some("serif".to_string()),
                    font_size_pt: Some(14),
                    text_color: Some("#1f2937".to_string()),
                    highlight_color: Some("#fff3bf".to_string()),
                },
                field: None,
                note_reference: None,
                tracked_change: None,
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("data-style=\"quote\""));
        assert!(html.contains("text-align:center"));
        assert!(html.contains("line-height:1.5"));
        assert!(html.contains("font-family:serif"));
        assert!(html.contains("background-color:#fff3bf"));
    }

    #[test]
    fn html_export_applies_paragraph_style_properties() {
        let mut document = Document::new_untitled();
        document
            .register_style(Style {
                id: "quote".into(),
                name: "Quote".to_string(),
                kind: StyleKind::Paragraph,
                parent: None,
                properties: word_core::StyleProperties {
                    paragraph: Some(word_core::ParagraphFormat {
                        alignment: Some(word_core::ParagraphAlignment::Justify),
                        line_spacing_per_mille: Some(1500),
                        spacing_before_mm: Some(0),
                        spacing_after_mm: Some(4),
                        indent_start_mm: Some(6),
                        indent_end_mm: None,
                        first_line_indent_mm: Some(-2),
                    }),
                    inline: None,
                    page: None,
                },
            })
            .expect("style should register");
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "quote".into(),
            format: Default::default(),
            inlines: vec![Inline::text("Styled")],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("data-style=\"quote\""));
        assert!(html.contains("text-align:justify"));
        assert!(html.contains("line-height:1.5"));
        assert!(html.contains("margin-top:0mm"));
        assert!(html.contains("margin-left:6mm"));
        assert!(html.contains("text-indent:-2mm"));
    }

    #[test]
    fn html_export_uses_list_definitions_for_ordered_lists() {
        let mut document = Document::new_untitled();
        document.lists.insert(
            "numbers".to_string(),
            ListDefinition {
                ordered: true,
                marker: None,
            },
        );
        document.sections[0].blocks = vec![Block::List(ListBlock {
            definition_id: "numbers".to_string(),
            items: vec![ListItem {
                level: 0,
                blocks: vec![Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: "body".into(),
                    format: Default::default(),
                    inlines: vec![Inline::text("First")],
                })],
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<ol><li><p data-style=\"body\">First</p></li></ol>"));
    }

    #[test]
    fn html_export_does_not_emit_remote_images_or_handlers() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "asset-1".to_string(),
            presentation: ImagePresentation::default(),
            alt_text: Some("<img src=x onerror=alert(1)>".to_string()),
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("data-asset=\"asset-1\""));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(!html.contains(" src=\"http"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn html_export_embeds_allowlisted_image_asset_as_data_url() {
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-1.png".to_string(),
            word_core::AssetRef {
                id: "image-1.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: 8,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Center,
                scale_percent: 75,
                caption: Some("Centered caption".to_string()),
            },
            alt_text: Some("Image".to_string()),
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"Image\">"));
        assert!(html.contains("data-align=\"center\""));
        assert!(html.contains("data-scale=\"75\""));
        assert!(html.contains("max-width:75%"));
        assert!(html.contains("<figure data-asset=\"image-1.png\" data-align=\"center\" data-scale=\"75\" style=\"margin-left:auto;margin-right:auto;max-width:75%;\">"));
        assert!(!html.contains("%;\"\">"));
        assert!(html.contains("<figcaption>Centered caption</figcaption>"));
        assert!(!html.contains("file://"));
        assert!(!html.contains("private"));
    }

    #[test]
    fn html_export_does_not_embed_mislabeled_image_asset() {
        let mut document = Document::new_untitled();
        document.assets.insert(
            "image-1.png".to_string(),
            word_core::AssetRef {
                id: "image-1.png".to_string(),
                media_type: "image/png".to_string(),
                byte_len: 4,
                bytes: b"HTML".to_vec(),
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "image-1.png".to_string(),
            presentation: ImagePresentation::default(),
            alt_text: Some("Image".to_string()),
        })];

        let html = export_html(&document).expect("html export should succeed");

        assert!(html.contains("<figure data-asset=\"image-1.png\""));
        assert!(!html.contains("<img"));
        assert!(!html.contains("data:image/"));
    }

    #[test]
    fn print_html_includes_page_setup_css() {
        let mut document = Document::new_untitled();
        document.sections[0].page.width_mm = 148;
        document.sections[0].page.height_mm = 210;

        let html = export_print_html(&document).expect("print html should export");

        assert!(html.contains("@page{size:148mm 210mm;"));
        assert!(html.contains("Content-Security-Policy"));
    }

    #[test]
    fn pdf_export_emits_uri_annotations_for_safe_external_text_links() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline("Website", "https://example.test/report")],
            }),
            Block::Heading(Heading {
                bookmark_id: None,
                level: 2,
                inlines: vec![linked_inline("Docs", "http://docs.example.test")],
            }),
            Block::List(ListBlock {
                definition_id: "unordered".to_string(),
                items: vec![ListItem {
                    level: 0,
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: None,
                        style: "body".into(),
                        format: Default::default(),
                        inlines: vec![linked_inline("Email", "mailto:team@example.test")],
                    })],
                }],
            }),
            Block::Table(Table {
                column_widths: Vec::new(),
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        presentation: Default::default(),
                        blocks: vec![Block::Paragraph(Paragraph {
                            bookmark_id: None,
                            style: "body".into(),
                            format: Default::default(),
                            inlines: vec![linked_inline("Cell link", "https://example.test/cell")],
                        })],
                    }],
                }],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&text), 4);
        assert!(text.contains("/Annots ["));
        assert!(text.contains("/Subtype /Link"));
        assert!(text.contains("/S /URI"));
        assert!(text.contains("/URI (https://example.test/report)"));
        assert!(text.contains("/URI (http://docs.example.test)"));
        assert!(text.contains("/URI (mailto:team@example.test)"));
        assert!(text.contains("/URI (https://example.test/cell)"));
        assert!(pdf_contains(&pdf, "Website"));
        assert!(pdf_contains(&pdf, "Docs"));
        assert!(pdf_contains(&pdf, "Email"));
        assert!(pdf_contains(&pdf, "Cell link"));
    }

    #[test]
    fn pdf_export_bounds_uri_annotations_to_text_run_rectangles() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![
                Inline::text("Before "),
                linked_inline("Website", "https://example.test/report"),
                Inline::text(" after"),
            ],
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);
        let rects = pdf_link_annotation_rects(&text);

        assert_eq!(rects.len(), 1);
        let [left, bottom, right, top] = rects[0];
        let width = right - left;
        let height = top - bottom;
        assert!(left > 80.0, "link rect should start near linked text");
        assert!(right < 180.0, "link rect should not span the page");
        assert!(
            (35.0..=70.0).contains(&width),
            "link rect width should roughly match the linked label"
        );
        assert!(
            (12.0..=18.0).contains(&height),
            "link rect height should roughly match one text line"
        );
        assert!(bottom > 0.0);
        assert!(top < mm_to_points(document.sections[0].page.height_mm));
    }

    #[test]
    fn pdf_export_emits_internal_destinations_for_safe_paragraph_and_heading_bookmarks() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Heading(Heading {
                bookmark_id: Some("bm-heading".to_string()),
                level: 2,
                inlines: vec![Inline::text("Heading target")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline("Jump to heading", "#bm-heading")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: Some("bm-paragraph".to_string()),
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("Paragraph target")],
            }),
            Block::Heading(Heading {
                bookmark_id: None,
                level: 3,
                inlines: vec![linked_inline("Jump to paragraph", "#bm-paragraph")],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&text), 2);
        assert_eq!(pdf_dest_annotation_count(&text), 2);
        assert!(text.contains("/Dest ["));
        assert!(!text.contains("/S /URI"));
        assert!(!text.contains("#bm-heading"));
        assert!(!text.contains("bm-heading"));
        assert!(!text.contains("#bm-paragraph"));
        assert!(!text.contains("bm-paragraph"));
        assert!(pdf_contains(&pdf, "Jump to heading"));
        assert!(pdf_contains(&pdf, "Jump to paragraph"));
        assert!(pdf_contains(&pdf, "Heading target"));
        assert!(pdf_contains(&pdf, "Paragraph target"));
    }

    #[test]
    fn pdf_export_emits_internal_destinations_for_generated_toc_entries() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::TableOfContents(TableOfContents {
                title: "Contents".to_string(),
                entries: vec![
                    TableOfContentsEntry {
                        level: 1,
                        text: "Intro".to_string(),
                        target_bookmark_id: "bm-intro".to_string(),
                    },
                    TableOfContentsEntry {
                        level: 2,
                        text: "Details".to_string(),
                        target_bookmark_id: "bm-details".to_string(),
                    },
                ],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-intro".to_string()),
                level: 1,
                inlines: vec![Inline::text("Intro")],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-details".to_string()),
                level: 2,
                inlines: vec![Inline::text("Details")],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&text), 2);
        assert_eq!(pdf_dest_annotation_count(&text), 2);
        assert!(!text.contains("/S /URI"));
        assert!(!text.contains("#bm-intro"));
        assert!(!text.contains("bm-intro"));
        assert!(!text.contains("#bm-details"));
        assert!(!text.contains("bm-details"));
        assert!(pdf_contains(&pdf, "Contents"));
        assert!(pdf_contains(&pdf, "Intro"));
        assert!(pdf_contains(&pdf, "Details"));
    }

    #[test]
    fn pdf_export_omits_generated_toc_destinations_for_duplicate_targets() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::TableOfContents(TableOfContents {
                title: "Contents".to_string(),
                entries: vec![TableOfContentsEntry {
                    level: 1,
                    text: "Duplicate".to_string(),
                    target_bookmark_id: "bm-duplicate".to_string(),
                }],
            }),
            Block::Heading(Heading {
                bookmark_id: Some("bm-duplicate".to_string()),
                level: 1,
                inlines: vec![Inline::text("Duplicate heading target")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: Some("bm-duplicate".to_string()),
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("Duplicate paragraph target")],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&text), 0);
        assert_eq!(pdf_dest_annotation_count(&text), 0);
        assert!(!text.contains("/Annots ["));
        assert!(!text.contains("#bm-duplicate"));
        assert!(!text.contains("bm-duplicate"));
        assert!(pdf_contains(&pdf, "Contents"));
        assert!(pdf_contains(&pdf, "Duplicate"));
        assert!(pdf_contains(&pdf, "Duplicate heading target"));
        assert!(pdf_contains(&pdf, "Duplicate paragraph target"));
    }

    #[test]
    fn pdf_export_omits_unsafe_missing_and_ambiguous_internal_annotations_without_leaking_hrefs() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Heading(Heading {
                bookmark_id: Some("bm-duplicate".to_string()),
                level: 2,
                inlines: vec![Inline::text("Duplicate heading target")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: Some("bm-duplicate".to_string()),
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("Duplicate paragraph target")],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![
                    linked_inline("Script label", "javascript:alert(1)"),
                    Inline::text(" "),
                    linked_inline("File label", "file:///local/private-report.pdf"),
                    Inline::text(" "),
                    linked_inline("Path label", "local/private-report.pdf"),
                    Inline::text(" "),
                    linked_inline("Unsafe fragment label", "#../bad"),
                    Inline::text(" "),
                    linked_inline("Missing label", "#bm-missing"),
                    Inline::text(" "),
                    linked_inline("Duplicate label", "#bm-duplicate"),
                ],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&text), 0);
        assert!(!text.contains("/Annots ["));
        assert!(!text.contains("javascript:"));
        assert!(!text.contains("file://"));
        assert!(!text.contains("local/private-report.pdf"));
        assert!(!text.contains("#../bad"));
        assert!(!text.contains("#bm-missing"));
        assert!(!text.contains("bm-missing"));
        assert!(!text.contains("#bm-duplicate"));
        assert!(!text.contains("bm-duplicate"));
        assert!(pdf_contains(&pdf, "Script label"));
        assert!(pdf_contains(&pdf, "File label"));
        assert!(pdf_contains(&pdf, "Path label"));
        assert!(pdf_contains(&pdf, "Unsafe fragment label"));
        assert!(pdf_contains(&pdf, "Missing label"));
        assert!(pdf_contains(&pdf, "Duplicate label"));
    }

    #[test]
    fn pdf_export_page_range_omits_internal_annotations_to_targets_outside_selected_pages() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline("Jump to later target", "#bm-later")],
            }),
            Block::PageBreak,
            Block::Heading(Heading {
                bookmark_id: Some("bm-later".to_string()),
                level: 2,
                inlines: vec![Inline::text("Later target")],
            }),
        ];

        let pdf = export_pdf_with_options(
            &document,
            PdfExportOptions {
                page_range: Some(PdfPageRange { start: 1, end: 1 }),
            },
        )
        .expect("valid page range should export");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&text), 1);
        assert_eq!(pdf_link_annotation_count(&text), 0);
        assert!(!text.contains("/Annots ["));
        assert!(!text.contains("#bm-later"));
        assert!(!text.contains("bm-later"));
        assert!(pdf_contains(&pdf, "Jump to later target"));
        assert!(!pdf_contains(&pdf, "Later target"));
    }

    #[test]
    fn pdf_export_internal_destinations_reference_shifted_page_objects() {
        let mut document = Document::new_untitled();
        let jpeg = tiny_real_jpeg();
        document.assets.insert(
            "page-one.jpg".to_string(),
            word_core::AssetRef {
                id: "page-one.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                byte_len: jpeg.len(),
                bytes: jpeg,
                original_name: None,
            },
        );
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline(
                    "External page-one link",
                    "https://example.test/one",
                )],
            }),
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline("Jump to page two", "#bm-page-two")],
            }),
            Block::Image(ImageBlock {
                asset_id: "page-one.jpg".to_string(),
                presentation: ImagePresentation {
                    alignment: ImageAlignment::Left,
                    scale_percent: 50,
                    caption: Some("Page one image caption".to_string()),
                },
                alt_text: Some("Page one image alt".to_string()),
            }),
            Block::PageBreak,
            Block::Heading(Heading {
                bookmark_id: Some("bm-page-two".to_string()),
                level: 2,
                inlines: vec![Inline::text("Page two target")],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);
        let page_objects = pdf_page_kid_object_numbers(&text);

        assert_eq!(page_objects.len(), 2);
        assert!(
            page_objects[1] > 6,
            "page two object should move after page-one annotations/images"
        );
        assert_eq!(pdf_image_xobject_count(&text), 1);
        assert_eq!(pdf_link_annotation_count(&text), 2);
        assert_eq!(pdf_dest_annotation_count(&text), 1);
        assert!(text.contains("/URI (https://example.test/one)"));
        assert!(text.contains(&format!("/Dest [{} 0 R /XYZ", page_objects[1])));
        assert!(!text.contains("#bm-page-two"));
        assert!(!text.contains("bm-page-two"));
        assert!(pdf_contains(&pdf, "Jump to page two"));
        assert!(pdf_contains(&pdf, "Page two target"));
    }

    #[test]
    fn pdf_export_emits_internal_destinations_for_list_and_table_targets() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![
                    linked_inline("Jump to list", "#bm-list-target"),
                    Inline::text(" "),
                    linked_inline("Jump to cell", "#bm-cell-target"),
                ],
            }),
            Block::List(ListBlock {
                definition_id: "unordered".to_string(),
                items: vec![ListItem {
                    level: 0,
                    blocks: vec![Block::Paragraph(Paragraph {
                        bookmark_id: Some("bm-list-target".to_string()),
                        style: "body".into(),
                        format: Default::default(),
                        inlines: vec![Inline::text("List bookmark target")],
                    })],
                }],
            }),
            Block::Table(Table {
                column_widths: Vec::new(),
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        presentation: Default::default(),
                        blocks: vec![Block::Paragraph(Paragraph {
                            bookmark_id: Some("bm-cell-target".to_string()),
                            style: "body".into(),
                            format: Default::default(),
                            inlines: vec![Inline::text("Cell bookmark target")],
                        })],
                    }],
                }],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_link_annotation_count(&text), 2);
        assert_eq!(pdf_dest_annotation_count(&text), 2);
        assert!(!text.contains("/S /URI"));
        assert!(!text.contains("#bm-list-target"));
        assert!(!text.contains("bm-list-target"));
        assert!(!text.contains("#bm-cell-target"));
        assert!(!text.contains("bm-cell-target"));
        assert!(pdf_contains(&pdf, "Jump to list"));
        assert!(pdf_contains(&pdf, "Jump to cell"));
        assert!(pdf_contains(&pdf, "List bookmark target"));
        assert!(pdf_contains(&pdf, "Cell bookmark target"));
    }

    #[test]
    fn pdf_export_bounds_mixed_uri_internal_and_toc_annotations_per_page() {
        let mut document = Document::new_untitled();
        document.sections[0].page = PageSetup {
            width_mm: 210,
            height_mm: 5000,
            margin_top_mm: 10,
            margin_right_mm: 10,
            margin_bottom_mm: 10,
            margin_left_mm: 10,
        };

        let target_count = PDF_MAX_LINK_ANNOTATIONS_PER_PAGE + 5;
        let mut blocks = (0..target_count)
            .map(|index| {
                Block::Heading(Heading {
                    bookmark_id: Some(format!("bm-cap-{index}")),
                    level: 2,
                    inlines: vec![Inline::text(format!("Cap target {index}"))],
                })
            })
            .collect::<Vec<_>>();
        blocks.push(Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![linked_inline(
                "External cap link",
                "https://example.test/cap",
            )],
        }));
        blocks.push(Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![linked_inline("Internal cap link", "#bm-cap-0")],
        }));
        blocks.push(Block::TableOfContents(TableOfContents {
            title: "Cap contents".to_string(),
            entries: (0..target_count)
                .map(|index| TableOfContentsEntry {
                    level: 1,
                    text: format!("Cap entry {index}"),
                    target_bookmark_id: format!("bm-cap-{index}"),
                })
                .collect(),
        }));
        document.sections[0].blocks = blocks;

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(
            pdf_link_annotation_count(&text),
            PDF_MAX_LINK_ANNOTATIONS_PER_PAGE
        );
        assert_eq!(text.matches("/S /URI").count(), 1);
        assert_eq!(
            pdf_dest_annotation_count(&text),
            PDF_MAX_LINK_ANNOTATIONS_PER_PAGE - 1
        );
        assert!(text.contains("/URI (https://example.test/cap)"));
        assert!(pdf_contains(&pdf, "Internal cap link"));
        assert!(pdf_contains(
            &pdf,
            &format!("Cap entry {}", target_count - 1)
        ));
        assert!(!text.contains("#bm-cap-0"));
        assert!(!text.contains("bm-cap-0"));
        assert!(!text.contains(&format!("#bm-cap-{}", target_count - 1)));
        assert!(!text.contains(&format!("bm-cap-{}", target_count - 1)));
    }

    #[test]
    fn pdf_export_bounds_link_annotations_per_page() {
        let mut document = Document::new_untitled();
        document.sections[0].page = PageSetup {
            width_mm: 210,
            height_mm: 1000,
            margin_top_mm: 10,
            margin_right_mm: 10,
            margin_bottom_mm: 10,
            margin_left_mm: 10,
        };
        document.sections[0].blocks = (0..(PDF_MAX_LINK_ANNOTATIONS_PER_PAGE + 10))
            .map(|index| {
                Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: "body".into(),
                    format: Default::default(),
                    inlines: vec![linked_inline(
                        format!("Link {index}"),
                        format!("https://example.test/{index}"),
                    )],
                })
            })
            .collect();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(
            pdf_link_annotation_count(&text),
            PDF_MAX_LINK_ANNOTATIONS_PER_PAGE
        );
        assert!(text.contains("/URI (https://example.test/0)"));
        assert!(text.contains(&format!(
            "/URI (https://example.test/{})",
            PDF_MAX_LINK_ANNOTATIONS_PER_PAGE - 1
        )));
        assert!(!text.contains(&format!(
            "/URI (https://example.test/{})",
            PDF_MAX_LINK_ANNOTATIONS_PER_PAGE
        )));
        assert!(pdf_contains(
            &pdf,
            &format!("Link {}", PDF_MAX_LINK_ANNOTATIONS_PER_PAGE)
        ));
    }

    #[test]
    fn pdf_export_bounds_link_annotations_per_document() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = (0..(PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT + 25))
            .map(|index| {
                Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: "body".into(),
                    format: Default::default(),
                    inlines: vec![linked_inline(
                        format!("Doc link {index}"),
                        format!("https://example.test/document/{index}"),
                    )],
                })
            })
            .collect();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(
            pdf_link_annotation_count(&text),
            PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT
        );
        assert!(text.contains("/URI (https://example.test/document/0)"));
        assert!(text.contains(&format!(
            "/URI (https://example.test/document/{})",
            PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT - 1
        )));
        assert!(!text.contains(&format!(
            "/URI (https://example.test/document/{})",
            PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT
        )));
        assert!(pdf_contains(
            &pdf,
            &format!("Doc link {}", PDF_MAX_LINK_ANNOTATIONS_PER_DOCUMENT)
        ));
    }

    #[test]
    fn pdf_export_page_range_keeps_annotations_for_selected_pages() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline("Page one link", "https://example.test/one")],
            }),
            Block::PageBreak,
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![linked_inline("Page two link", "https://example.test/two")],
            }),
        ];

        let pdf = export_pdf_with_options(
            &document,
            PdfExportOptions {
                page_range: Some(PdfPageRange { start: 2, end: 2 }),
            },
        )
        .expect("valid range should export");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&text), 1);
        assert_eq!(pdf_link_annotation_count(&text), 1);
        assert!(!text.contains("https://example.test/one"));
        assert!(text.contains("/URI (https://example.test/two)"));
        assert!(!pdf_contains(&pdf, "Page one link"));
        assert!(pdf_contains(&pdf, "Page two link"));
    }

    #[test]
    fn pdf_export_paginates_into_multiple_page_objects() {
        let mut document = Document::new_untitled();
        document.sections[0].page = compact_test_page();
        document.sections[0].blocks = (0..36)
            .map(|index| {
                Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: "body".into(),
                    format: Default::default(),
                    inlines: vec![Inline::text(format!("Line {index}"))],
                })
            })
            .collect();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert!(pdf_page_object_count(&text) > 1);
        assert!(text.contains("/Type /Pages"));
        assert!(text.contains("/Kids ["));
        assert!(text.contains(&format!("/Count {}", pdf_page_object_count(&text))));
    }

    #[test]
    fn pdf_export_renders_table_borders_and_cell_text() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: Vec::new(),
            rows: vec![
                TableRow {
                    cells: vec![
                        TableCell {
                            presentation: Default::default(),
                            blocks: vec![paragraph_block("Header cell")],
                        },
                        TableCell {
                            presentation: Default::default(),
                            blocks: vec![paragraph_block("Wrapped table cell text")],
                        },
                    ],
                },
                TableRow {
                    cells: vec![
                        TableCell {
                            presentation: Default::default(),
                            blocks: vec![paragraph_block("First value")],
                        },
                        TableCell {
                            presentation: Default::default(),
                            blocks: vec![paragraph_block("Second value")],
                        },
                    ],
                },
            ],
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&text), 1);
        assert!(text.contains("/Type /Pages"));
        assert!(text.contains(" re S"));
        assert!(text.contains("0.6 w"));
        assert!(pdf_contains(&pdf, "Header cell"));
        assert!(pdf_contains(&pdf, "Wrapped table cell text"));
        assert!(pdf_contains(&pdf, "Second value"));
    }

    #[test]
    fn table_cell_styling_exports_to_html_print_html_and_pdf_projection() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: Vec::new(),
            rows: vec![TableRow {
                cells: vec![TableCell {
                    presentation: TableCellPresentation {
                        background_color: Some("#dbeafe".to_string()),
                        text_alignment: Some(ParagraphAlignment::Right),
                        border: TableCellBorder::Hidden,
                    },
                    blocks: vec![paragraph_block("Styled cell")],
                }],
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");
        let print_html = export_print_html(&document).expect("print html should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert!(html.contains("data-cell-background-color=\"#dbeafe\""));
        assert!(html.contains("background-color:#dbeafe"));
        assert!(html.contains("data-cell-align=\"right\""));
        assert!(html.contains("text-align:right"));
        assert!(html.contains("data-cell-border=\"hidden\""));
        assert!(html.contains("border-color:transparent"));
        assert!(print_html.contains("data-cell-background-color=\"#dbeafe\""));
        assert!(pdf_text.contains("0.859 0.918 0.996 rg"));
        assert!(!pdf_text.contains(" re S"));
        assert!(pdf_contains(&pdf, "Styled cell"));
    }

    #[test]
    fn table_column_widths_export_to_html_print_html_and_pdf_layout() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: vec![250, 750],
            rows: vec![TableRow {
                cells: vec![
                    TableCell {
                        presentation: Default::default(),
                        blocks: vec![paragraph_block("Narrow")],
                    },
                    TableCell {
                        presentation: Default::default(),
                        blocks: vec![paragraph_block("Wide")],
                    },
                ],
            }],
        })];

        let html = export_html(&document).expect("html export should succeed");
        let print_html = export_print_html(&document).expect("print html should succeed");
        let layout = layout_pdf_table_row(
            &PdfProjectedTableRow {
                section_index: 0,
                column_widths: vec![250, 750],
                cells: vec![
                    PdfProjectedTableCell {
                        text: PdfLinkedText::from_plain("Narrow"),
                        presentation: Default::default(),
                    },
                    PdfProjectedTableCell {
                        text: PdfLinkedText::from_plain("Wide"),
                        presentation: Default::default(),
                    },
                ],
            },
            400.0,
        );

        assert!(html.contains("data-column-widths=\"250,750\""));
        assert!(html.contains("<col style=\"width:25%\">"));
        assert!(html.contains("<col style=\"width:75%\">"));
        assert!(!html.contains("file://"));
        assert!(!html.contains("private"));
        assert!(print_html.contains("data-column-widths=\"250,750\""));
        assert_eq!(layout.cell_widths, vec![100.0, 300.0]);

        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: vec![10, 990],
            rows: vec![TableRow {
                cells: vec![
                    TableCell {
                        presentation: Default::default(),
                        blocks: vec![paragraph_block("A")],
                    },
                    TableCell {
                        presentation: Default::default(),
                        blocks: vec![paragraph_block("B")],
                    },
                ],
            }],
        })];
        let invalid_html = export_html(&document).expect("html export should succeed");
        assert!(!invalid_html.contains("data-column-widths"));
        assert!(!invalid_html.contains("<colgroup>"));
    }

    #[test]
    fn pdf_table_width_fallback_handles_oversized_column_counts() {
        let widths = pdf_table_cell_widths(&[], usize::from(u16::MAX) + 1, 65_536.0);

        assert_eq!(widths.len(), usize::from(u16::MAX) + 1);
        assert_eq!(widths[0], 1.0);
        assert_eq!(widths[usize::from(u16::MAX)], 1.0);
    }

    #[test]
    fn pdf_export_paginates_structured_table_and_figure_blocks() {
        let mut document = Document::new_untitled();
        document.sections[0].page = compact_test_page();
        let mut rows = Vec::new();
        for index in 0..12 {
            rows.push(TableRow {
                cells: vec![TableCell {
                    presentation: Default::default(),
                    blocks: vec![paragraph_block(format!("Structured row {index}"))],
                }],
            });
        }
        document.sections[0].blocks = vec![
            Block::Table(Table {
                column_widths: Vec::new(),
                rows,
            }),
            Block::Image(ImageBlock {
                asset_id: "structured-figure-asset".to_string(),
                presentation: ImagePresentation {
                    alignment: ImageAlignment::Center,
                    scale_percent: 75,
                    caption: Some("Structured figure caption".to_string()),
                },
                alt_text: Some("Structured figure alt".to_string()),
            }),
        ];

        let pages = paginate_pdf(&document).expect("pdf pagination should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert!(pages.len() > 1);
        assert_eq!(pdf_page_object_count(&text), pages.len());
        assert!(pdf_contains(&pdf, "Structured row 0"));
        assert!(pdf_contains(&pdf, "Structured row 11"));
        assert!(pdf_contains(&pdf, "Structured figure alt"));
        assert!(pdf_contains(&pdf, "Structured figure caption"));
    }

    #[test]
    fn pdf_export_splits_oversized_table_rows_across_pages() {
        let mut document = Document::new_untitled();
        document.sections[0].page = compact_test_page();
        let long_cell_text = (0..120)
            .map(|index| format!("cell{index:03}"))
            .collect::<Vec<_>>()
            .join(" ");
        document.sections[0].blocks = vec![Block::Table(Table {
            column_widths: Vec::new(),
            rows: vec![TableRow {
                cells: vec![TableCell {
                    presentation: Default::default(),
                    blocks: vec![paragraph_block(long_cell_text)],
                }],
            }],
        })];

        let pages = paginate_pdf(&document).expect("pdf pagination should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert!(pages.len() > 1);
        assert_eq!(pdf_page_object_count(&text), pages.len());
        assert!(text.contains(" re S"));
        assert!(pdf_contains(&pdf, "cell000"));
        assert!(pdf_contains(&pdf, "cell119"));
    }

    #[test]
    fn pdf_export_splits_oversized_figure_placeholders_across_pages() {
        let mut document = Document::new_untitled();
        document.sections[0].page = compact_test_page();
        let long_alt_text = (0..120)
            .map(|index| format!("figure{index:03}"))
            .collect::<Vec<_>>()
            .join(" ");
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "private-source/figure.png".to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Right,
                scale_percent: 100,
                caption: Some("Public caption".to_string()),
            },
            alt_text: Some(long_alt_text),
        })];

        let pages = paginate_pdf(&document).expect("pdf pagination should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert!(pages.len() > 1);
        assert_eq!(pdf_page_object_count(&text), pages.len());
        assert!(text.contains(" re S"));
        assert!(pdf_contains(&pdf, "figure000"));
        assert!(pdf_contains(&pdf, "figure119"));
        assert!(pdf_contains(&pdf, "Public caption"));
        assert!(!text.contains("private-source"));
        assert!(!text.contains("figure.png"));
    }

    #[test]
    fn pdf_export_honors_explicit_page_breaks() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("Before break")],
            }),
            Block::PageBreak,
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("After break")],
            }),
        ];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&text), 2);
        assert!(pdf_contains(&pdf, "Before break"));
        assert!(pdf_contains(&pdf, "After break"));
    }

    #[test]
    fn pdf_export_keeps_section_page_setup_boundaries() {
        let mut document = Document::new_untitled();
        let mut first_section = Section {
            page: PageSetup {
                width_mm: 80,
                height_mm: 80,
                margin_top_mm: 10,
                margin_right_mm: 10,
                margin_bottom_mm: 10,
                margin_left_mm: 10,
            },
            ..Section::default()
        };
        first_section.blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![Inline::text("First section")],
        })];
        let mut second_section = Section {
            page: PageSetup {
                width_mm: 120,
                height_mm: 100,
                margin_top_mm: 10,
                margin_right_mm: 10,
                margin_bottom_mm: 10,
                margin_left_mm: 10,
            },
            ..Section::default()
        };
        second_section.blocks = vec![Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![Inline::text("Second section")],
        })];
        document.sections = vec![first_section, second_section];

        let pages = paginate_pdf(&document).expect("pdf pagination should succeed");
        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].section_index, 0);
        assert_eq!(pages[0].page_setup.width_mm, 80);
        assert_eq!(pages[1].section_index, 1);
        assert_eq!(pages[1].page_setup.width_mm, 120);
        assert!(text.contains("/MediaBox [0 0 226.8 226.8]"));
        assert!(text.contains("/MediaBox [0 0 340.2 283.5]"));
        assert!(pdf_contains(&pdf, "First section"));
        assert!(pdf_contains(&pdf, "Second section"));
    }

    #[test]
    fn pdf_export_renders_header_footer_fields_deterministically() {
        let mut document = Document::new_untitled();
        document.sections[0].page = compact_test_page();
        document.sections[0].page_regions.header.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![
                    Inline::text("Page "),
                    Inline::field(PageField::PageNumber),
                    Inline::text(" of "),
                    Inline::field(PageField::PageCount),
                ],
            })];
        document.sections[0].page_regions.footer.blocks =
            vec![PageRegionBlock::Paragraph(PageRegionParagraph {
                inlines: vec![Inline::text("Updated "), Inline::field(PageField::Date)],
            })];
        document.sections[0].blocks = (0..24)
            .map(|index| {
                Block::Paragraph(Paragraph {
                    bookmark_id: None,
                    style: "body".into(),
                    format: Default::default(),
                    inlines: vec![Inline::text(format!("Body {index}"))],
                })
            })
            .collect();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);
        let page_count = pdf_page_object_count(&text);
        let expected_date = document.meta.modified_at.format("%Y-%m-%d").to_string();

        assert!(page_count > 1);
        assert!(pdf_contains(&pdf, &format!("Page 1 of {page_count}")));
        assert!(pdf_contains(
            &pdf,
            &format!("Page {page_count} of {page_count}")
        ));
        assert!(pdf_contains(&pdf, &format!("Updated {expected_date}")));
    }

    #[test]
    fn pdf_export_validates_page_ranges() {
        let mut document = Document::new_untitled();
        document.sections[0].blocks = vec![
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("Page one")],
            }),
            Block::PageBreak,
            Block::Paragraph(Paragraph {
                bookmark_id: None,
                style: "body".into(),
                format: Default::default(),
                inlines: vec![Inline::text("Page two")],
            }),
        ];

        let one_page = export_pdf_with_options(
            &document,
            PdfExportOptions {
                page_range: Some(PdfPageRange { start: 2, end: 2 }),
            },
        )
        .expect("valid range should export");
        let text = String::from_utf8_lossy(&one_page);
        assert_eq!(pdf_page_object_count(&text), 1);
        assert!(!pdf_contains(&one_page, "Page one"));
        assert!(pdf_contains(&one_page, "Page two"));

        assert_eq!(
            export_pdf_with_options(
                &document,
                PdfExportOptions {
                    page_range: Some(PdfPageRange { start: 0, end: 1 }),
                },
            )
            .expect_err("zero range should fail"),
            ExportError::InvalidPdfPageRange
        );
        assert_eq!(
            export_pdf_with_options(
                &document,
                PdfExportOptions {
                    page_range: Some(PdfPageRange { start: 3, end: 3 }),
                },
            )
            .expect_err("empty range should fail"),
            ExportError::InvalidPdfPageRange
        );
        assert_eq!(
            export_pdf_with_options(
                &document,
                PdfExportOptions {
                    page_range: Some(PdfPageRange { start: 2, end: 1 }),
                },
            )
            .expect_err("inverted range should fail"),
            ExportError::InvalidPdfPageRange
        );
    }

    #[test]
    fn pdf_export_omits_private_metadata_and_local_paths() {
        let mut document = Document::new_untitled();
        let private_asset_id = ["file://", "local-source/", "figure.png"].concat();
        let private_original_name = "private-source-name.png";
        let private_user = "operator-name";
        let private_host = "workstation-name";
        document.meta.title = format!("{private_user} {private_host}");
        document.assets.insert(
            private_asset_id.clone(),
            word_core::AssetRef {
                id: private_asset_id.clone(),
                media_type: "image/png".to_string(),
                byte_len: 8,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                original_name: Some(private_original_name.to_string()),
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: private_asset_id.clone(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Center,
                scale_percent: 100,
                caption: Some("Generic caption".to_string()),
            },
            alt_text: Some("Generic image".to_string()),
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert!(!text.contains(&private_asset_id));
        assert!(!text.contains(private_original_name));
        assert!(!text.contains(private_user));
        assert!(!text.contains(private_host));
        assert!(!text.contains("local-source"));
        assert!(!text.contains("file://"));
        assert!(!text.contains("CreationDate"));
        assert!(!text.contains("Producer"));
        assert!(text.contains(" re S"));
        assert!(pdf_contains(&pdf, "Generic image"));
        assert!(pdf_contains(&pdf, "Generic caption"));
    }

    #[test]
    fn pdf_export_embeds_safe_jpeg_images_as_dct_xobjects() {
        let mut document = Document::new_untitled();
        let private_asset_id = "private-source/client-photo.jpg";
        let jpeg = tiny_real_jpeg();
        document.assets.insert(
            private_asset_id.to_string(),
            word_core::AssetRef {
                id: private_asset_id.to_string(),
                media_type: "image/jpeg".to_string(),
                byte_len: jpeg.len(),
                bytes: jpeg,
                original_name: Some("client-photo.jpg".to_string()),
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: private_asset_id.to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Center,
                scale_percent: 75,
                caption: Some("Visible JPEG caption".to_string()),
            },
            alt_text: Some("Visible JPEG alt".to_string()),
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_image_xobject_count(&text), 1);
        assert!(text.contains("/Subtype /Image"));
        assert!(text.contains("/Filter /DCTDecode"));
        assert!(text.contains("/ColorSpace /DeviceRGB"));
        assert!(text.contains("/XObject << /Im1"));
        assert!(text.contains("/Im1 Do"));
        assert!(pdf_contains(&pdf, "Visible JPEG alt"));
        assert!(pdf_contains(&pdf, "Visible JPEG caption"));
        assert!(!text.contains(private_asset_id));
        assert!(!text.contains("client-photo"));
        assert!(!text.contains("private-source"));
    }

    #[test]
    fn pdf_export_strips_jpeg_metadata_segments_before_embedding() {
        let mut document = Document::new_untitled();
        let private_metadata = "PRIVATE-JPEG-METADATA local/user/photo.jpg";
        let jpeg = jpeg_with_metadata_segment(&tiny_real_jpeg(), 0xe1, private_metadata.as_bytes());
        document.assets.insert(
            "metadata-bearing-image.jpg".to_string(),
            word_core::AssetRef {
                id: "metadata-bearing-image.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                byte_len: jpeg.len(),
                bytes: jpeg,
                original_name: Some("photo.jpg".to_string()),
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: "metadata-bearing-image.jpg".to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Left,
                scale_percent: 100,
                caption: Some("Metadata-stripped caption".to_string()),
            },
            alt_text: Some("Metadata-stripped alt".to_string()),
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_image_xobject_count(&text), 1);
        assert!(text.contains("/Filter /DCTDecode"));
        assert!(pdf_contains(&pdf, "Metadata-stripped caption"));
        assert!(!text.contains(private_metadata));
        assert!(!text.contains("metadata-bearing-image"));
        assert!(!text.contains("photo.jpg"));
    }

    #[test]
    fn pdf_export_keeps_non_jpeg_images_on_visible_placeholder_path() {
        let mut document = Document::new_untitled();
        let private_asset_id = "private-source/figure.png";
        document.assets.insert(
            private_asset_id.to_string(),
            word_core::AssetRef {
                id: private_asset_id.to_string(),
                media_type: "image/png".to_string(),
                byte_len: 8,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                original_name: Some("figure.png".to_string()),
            },
        );
        document.sections[0].blocks = vec![Block::Image(ImageBlock {
            asset_id: private_asset_id.to_string(),
            presentation: ImagePresentation {
                alignment: ImageAlignment::Right,
                scale_percent: 80,
                caption: Some("PNG placeholder caption".to_string()),
            },
            alt_text: Some("PNG placeholder alt".to_string()),
        })];

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_image_xobject_count(&text), 0);
        assert!(!text.contains("/DCTDecode"));
        assert!(text.contains(" re S"));
        assert!(pdf_contains(&pdf, "PNG placeholder alt"));
        assert!(pdf_contains(&pdf, "PNG placeholder caption"));
        assert!(!text.contains(private_asset_id));
        assert!(!text.contains("figure.png"));
        assert!(!text.contains("private-source"));
    }

    #[test]
    fn pdf_export_rejects_malformed_oversized_and_unsupported_jpegs_without_leaks() {
        let mut document = Document::new_untitled();
        let malformed = b"\xff\xd8\xff\xe0\x00\x10RAW-PRIVATE-JPEG-PAYLOAD".to_vec();
        let corrupt_after_sof = corrupt_after_sof_jpeg();
        let mut oversized = tiny_real_jpeg();
        oversized.resize(PDF_MAX_JPEG_BYTES_PER_IMAGE + 1, b'X');
        oversized.extend_from_slice(b"OVERSIZED-PRIVATE-JPEG-PAYLOAD");
        let oversized_dimensions = synthetic_jpeg((PDF_MAX_JPEG_DIMENSION + 1) as u16, 1, 3);
        let oversized_pixels = synthetic_jpeg(5000, 5000, 3);
        let unsupported_components = synthetic_jpeg(4, 1, 4);
        let assets = [
            (
                "private-malformed-asset.jpg",
                malformed,
                "Malformed JPEG caption",
            ),
            (
                "private-corrupt-after-sof-asset.jpg",
                corrupt_after_sof,
                "Corrupt JPEG caption",
            ),
            (
                "private-oversized-asset.jpg",
                oversized,
                "Oversized JPEG caption",
            ),
            (
                "private-dimension-asset.jpg",
                oversized_dimensions,
                "Oversized dimension caption",
            ),
            (
                "private-pixels-asset.jpg",
                oversized_pixels,
                "Oversized pixels caption",
            ),
            (
                "private-cmyk-asset.jpg",
                unsupported_components,
                "Unsupported JPEG caption",
            ),
        ];
        for (asset_id, bytes, _) in &assets {
            document.assets.insert(
                (*asset_id).to_string(),
                word_core::AssetRef {
                    id: (*asset_id).to_string(),
                    media_type: "image/jpeg".to_string(),
                    byte_len: bytes.len(),
                    bytes: bytes.clone(),
                    original_name: Some((*asset_id).to_string()),
                },
            );
        }
        document.sections[0].blocks = assets
            .iter()
            .map(|(asset_id, _, caption)| {
                Block::Image(ImageBlock {
                    asset_id: (*asset_id).to_string(),
                    presentation: ImagePresentation {
                        alignment: ImageAlignment::Left,
                        scale_percent: 100,
                        caption: Some((*caption).to_string()),
                    },
                    alt_text: Some("Public fallback alt".to_string()),
                })
            })
            .collect();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_image_xobject_count(&text), 0);
        assert!(!text.contains("/DCTDecode"));
        assert!(pdf_contains(&pdf, "Malformed JPEG caption"));
        assert!(pdf_contains(&pdf, "Corrupt JPEG caption"));
        assert!(pdf_contains(&pdf, "Oversized JPEG caption"));
        assert!(pdf_contains(&pdf, "Oversized dimension caption"));
        assert!(pdf_contains(&pdf, "Oversized pixels caption"));
        assert!(pdf_contains(&pdf, "Unsupported JPEG caption"));
        assert!(!text.contains("private-malformed-asset"));
        assert!(!text.contains("private-corrupt-after-sof-asset"));
        assert!(!text.contains("private-oversized-asset"));
        assert!(!text.contains("private-dimension-asset"));
        assert!(!text.contains("private-pixels-asset"));
        assert!(!text.contains("private-cmyk-asset"));
        assert!(!text.contains("RAW-PRIVATE-JPEG-PAYLOAD"));
        assert!(!text.contains("CORRUPT-PRIVATE-AFTER-SOF"));
        assert!(!text.contains("OVERSIZED-PRIVATE-JPEG-PAYLOAD"));
    }

    #[test]
    fn pdf_export_page_range_includes_only_selected_page_images() {
        let mut document = Document::new_untitled();
        for asset_id in ["page-one.jpg", "page-two.jpg"] {
            let jpeg = tiny_real_jpeg();
            document.assets.insert(
                asset_id.to_string(),
                word_core::AssetRef {
                    id: asset_id.to_string(),
                    media_type: "image/jpeg".to_string(),
                    byte_len: jpeg.len(),
                    bytes: jpeg,
                    original_name: None,
                },
            );
        }
        document.sections[0].blocks = vec![
            Block::Image(ImageBlock {
                asset_id: "page-one.jpg".to_string(),
                presentation: ImagePresentation {
                    alignment: ImageAlignment::Left,
                    scale_percent: 50,
                    caption: Some("First page image".to_string()),
                },
                alt_text: Some("First page alt".to_string()),
            }),
            Block::PageBreak,
            Block::Image(ImageBlock {
                asset_id: "page-two.jpg".to_string(),
                presentation: ImagePresentation {
                    alignment: ImageAlignment::Left,
                    scale_percent: 50,
                    caption: Some("Second page image".to_string()),
                },
                alt_text: Some("Second page alt".to_string()),
            }),
        ];

        let pdf = export_pdf_with_options(
            &document,
            PdfExportOptions {
                page_range: Some(PdfPageRange { start: 2, end: 2 }),
            },
        )
        .expect("valid page range should export");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(pdf_page_object_count(&text), 1);
        assert_eq!(pdf_image_xobject_count(&text), 1);
        assert!(!pdf_contains(&pdf, "First page image"));
        assert!(!pdf_contains(&pdf, "First page alt"));
        assert!(pdf_contains(&pdf, "Second page image"));
        assert!(pdf_contains(&pdf, "Second page alt"));
    }

    #[test]
    fn pdf_export_falls_back_after_embedded_image_cap() {
        let mut document = Document::new_untitled();
        document.sections[0].page = PageSetup {
            width_mm: 210,
            height_mm: 5000,
            margin_top_mm: 10,
            margin_right_mm: 10,
            margin_bottom_mm: 10,
            margin_left_mm: 10,
        };
        document.sections[0].blocks = (0..(PDF_MAX_EMBEDDED_IMAGES_PER_DOCUMENT + 2))
            .map(|index| {
                let asset_id = format!("image-{index}.jpg");
                let jpeg = tiny_real_jpeg();
                document.assets.insert(
                    asset_id.clone(),
                    word_core::AssetRef {
                        id: asset_id.clone(),
                        media_type: "image/jpeg".to_string(),
                        byte_len: jpeg.len(),
                        bytes: jpeg,
                        original_name: None,
                    },
                );
                Block::Image(ImageBlock {
                    asset_id,
                    presentation: ImagePresentation {
                        alignment: ImageAlignment::Left,
                        scale_percent: 50,
                        caption: Some(format!("Capped figure {index}")),
                    },
                    alt_text: Some(format!("Capped alt {index}")),
                })
            })
            .collect();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");
        let text = String::from_utf8_lossy(&pdf);

        assert_eq!(
            pdf_image_xobject_count(&text),
            PDF_MAX_EMBEDDED_IMAGES_PER_DOCUMENT
        );
        assert!(text.contains("/Im32 Do"));
        assert!(!text.contains("/Im33 Do"));
        assert!(pdf_contains(
            &pdf,
            &format!("Capped figure {}", PDF_MAX_EMBEDDED_IMAGES_PER_DOCUMENT)
        ));
        assert!(pdf_contains(
            &pdf,
            &format!("Capped alt {}", PDF_MAX_EMBEDDED_IMAGES_PER_DOCUMENT + 1)
        ));
    }

    #[test]
    fn pdf_export_returns_pdf_header() {
        let document = Document::new_untitled();

        let pdf = export_basic_pdf(&document).expect("pdf export should succeed");

        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert!(pdf.windows(4).any(|window| window == b"xref"));
        assert!(!pdf
            .windows("Start writing...".len())
            .any(|window| window == b"Start writing..."));
    }

    fn compact_test_page() -> PageSetup {
        PageSetup {
            width_mm: 80,
            height_mm: 80,
            margin_top_mm: 10,
            margin_right_mm: 10,
            margin_bottom_mm: 10,
            margin_left_mm: 10,
        }
    }

    fn paragraph_block(text: impl Into<String>) -> Block {
        Block::Paragraph(Paragraph {
            bookmark_id: None,
            style: "body".into(),
            format: Default::default(),
            inlines: vec![Inline::text(text)],
        })
    }

    fn linked_inline(text: impl Into<String>, href: impl Into<String>) -> Inline {
        Inline {
            text: text.into(),
            marks: Vec::new(),
            link: Some(href.into()),
            comment_ids: Vec::new(),
            style: Default::default(),
            field: None,
            note_reference: None,
            tracked_change: None,
        }
    }

    fn pdf_page_object_count(pdf: &str) -> usize {
        pdf.matches("/Type /Page ").count()
    }

    fn pdf_link_annotation_count(pdf: &str) -> usize {
        pdf.matches("/Subtype /Link").count()
    }

    fn pdf_dest_annotation_count(pdf: &str) -> usize {
        pdf.matches("/Dest [").count()
    }

    fn pdf_image_xobject_count(pdf: &str) -> usize {
        pdf.matches("/Subtype /Image").count()
    }

    fn pdf_page_kid_object_numbers(pdf: &str) -> Vec<usize> {
        let Some(start) = pdf.find("/Kids [") else {
            return Vec::new();
        };
        let remaining = &pdf[start + "/Kids [".len()..];
        let Some(end) = remaining.find(']') else {
            return Vec::new();
        };
        remaining[..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .chunks(3)
            .filter_map(|chunk| match chunk {
                [object_number, "0", "R"] => object_number.parse::<usize>().ok(),
                _ => None,
            })
            .collect()
    }

    fn pdf_link_annotation_rects(pdf: &str) -> Vec<[f32; 4]> {
        let mut rects = Vec::new();
        let mut remaining = pdf;
        while let Some(index) = remaining.find("/Rect [") {
            remaining = &remaining[index + "/Rect [".len()..];
            let Some(end) = remaining.find(']') else {
                break;
            };
            let numbers = remaining[..end]
                .split_whitespace()
                .filter_map(|part| part.parse::<f32>().ok())
                .collect::<Vec<_>>();
            if numbers.len() == 4 {
                rects.push([numbers[0], numbers[1], numbers[2], numbers[3]]);
            }
            remaining = &remaining[end + 1..];
        }
        rects
    }

    fn pdf_contains(pdf: &[u8], needle: &str) -> bool {
        pdf.windows(needle.len())
            .any(|window| window == needle.as_bytes())
    }

    fn synthetic_jpeg(width_px: u16, height_px: u16, components: u8) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend_from_slice(&[
            0xff, 0xe0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x00,
        ]);

        let sof_length = 8 + components as u16 * 3;
        bytes.extend_from_slice(&[0xff, 0xc0]);
        bytes.extend_from_slice(&sof_length.to_be_bytes());
        bytes.push(8);
        bytes.extend_from_slice(&height_px.to_be_bytes());
        bytes.extend_from_slice(&width_px.to_be_bytes());
        bytes.push(components);
        for component in 1..=components {
            bytes.extend_from_slice(&[component, 0x11, 0x00]);
        }

        let sos_length = 6 + components as u16 * 2;
        bytes.extend_from_slice(&[0xff, 0xda]);
        bytes.extend_from_slice(&sos_length.to_be_bytes());
        bytes.push(components);
        for component in 1..=components {
            bytes.extend_from_slice(&[component, 0x00]);
        }
        bytes.extend_from_slice(&[0x00, 0x3f, 0x00, 0x00, 0xff, 0xd9]);
        bytes
    }

    fn tiny_real_jpeg() -> Vec<u8> {
        decode_base64(
            "/9j/wAARCAABAAEDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9sAQwACAgICAgIDAgIDBQMDAwUGBQUFBQYIBgYGBgYICggICAgICAoKCgoKCgoKDAwMDAwMDg4ODg4PDw8PDw8PDw8P/9sAQwECAgIEBAQHBAQHEAsJCxAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ/90ABAAB/9oADAMBAAIRAxEAPwD1iiiiv8rz/FM//9k=",
        )
    }

    fn jpeg_with_metadata_segment(jpeg: &[u8], marker: u8, payload: &[u8]) -> Vec<u8> {
        assert!(jpeg.starts_with(b"\xff\xd8"));
        assert!((0xe0..=0xef).contains(&marker) || marker == 0xfe);
        let segment_length = payload.len() + 2;
        assert!(segment_length <= u16::MAX as usize);
        let mut output = Vec::with_capacity(jpeg.len() + payload.len() + 4);
        output.extend_from_slice(&jpeg[..2]);
        output.extend_from_slice(&[0xff, marker]);
        output.extend_from_slice(&(segment_length as u16).to_be_bytes());
        output.extend_from_slice(payload);
        output.extend_from_slice(&jpeg[2..]);
        output
    }

    fn corrupt_after_sof_jpeg() -> Vec<u8> {
        let mut jpeg = tiny_real_jpeg();
        let sos_index =
            jpeg_marker_index(&jpeg, 0xda).expect("tiny fixture should have SOS marker");
        jpeg.truncate(sos_index);
        jpeg.extend_from_slice(b"CORRUPT-PRIVATE-AFTER-SOF");
        jpeg.extend_from_slice(b"\xff\xd9");
        jpeg
    }

    fn jpeg_marker_index(jpeg: &[u8], marker: u8) -> Option<usize> {
        jpeg.windows(2).position(|window| window == [0xff, marker])
    }

    fn decode_base64(input: &str) -> Vec<u8> {
        let mut output = Vec::with_capacity(input.len() / 4 * 3);
        for chunk in input.as_bytes().chunks(4) {
            assert_eq!(chunk.len(), 4);
            let values = [
                base64_test_value(chunk[0]),
                base64_test_value(chunk[1]),
                base64_test_value(chunk[2]),
                base64_test_value(chunk[3]),
            ];
            let first = values[0].expect("base64 character");
            let second = values[1].expect("base64 character");
            let third = values[2].unwrap_or(0);
            let fourth = values[3].unwrap_or(0);
            output.push((first << 2) | (second >> 4));
            if chunk[2] != b'=' {
                output.push((second << 4) | (third >> 2));
            }
            if chunk[3] != b'=' {
                output.push((third << 6) | fourth);
            }
        }
        output
    }

    fn base64_test_value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => None,
            _ => panic!("unexpected base64 byte"),
        }
    }
}
