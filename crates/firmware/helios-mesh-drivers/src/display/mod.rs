/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// One generic display engine over `embedded-graphics` draw targets.

/* === Modules === */

pub mod oled;
pub mod tft;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::fmt::Write as _;

// Third Party Imports
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::{Receiver as WatchReceiver, Watch};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font::{MonoFont, MonoTextStyle},
    pixelcolor::PixelColor,
    prelude::{DrawTarget, Primitive},
    primitives::{Line, PrimitiveStyle},
    text::{Baseline, Text, TextStyleBuilder},
};

// Local Imports

/* === Constants === */

pub const MAX_STATUS_ROWS: usize = 7;
pub const MAX_NEARBY_PAGES: usize = 8;
pub const TITLE_LENGTH: usize = 32;
pub const LABEL_LENGTH: usize = 16;
pub const VALUE_LENGTH: usize = 24;
pub const SNAPSHOT_CONSUMERS: usize = 1;

// Capacity for a fully composed page before any panel-geometry truncation.
// `compose` builds a local page from `2 + MAX_STATUS_ROWS` rows (`Last
// Request`/`Last Response` plus every status row) and a nearby page from
// `3 + MAX_STATUS_ROWS` rows (`Last Seen` plus every status row plus the
// same two activity rows) — both share `MAX_STATUS_ROWS` for their status
// rows today, so the nearby shape's `+3` is this constant's bound.
// Composition never silently overflows this; `compose` separately
// truncates to what the panel can actually show.
const MAX_PAGE_ROWS: usize = MAX_STATUS_ROWS + 3;
const INPUT_POLL: Duration = Duration::from_millis(50);
const LOCAL_BADGE: &str = "Ready";
const NEARBY_BADGE: &str = "Nearby";

/* === Macros === */

/* === Type Aliases === */

pub type Title = heapless::String<TITLE_LENGTH>;
pub type Label = heapless::String<LABEL_LENGTH>;
pub type Value = heapless::String<VALUE_LENGTH>;
pub type SnapshotWatch = Watch<CriticalSectionRawMutex, Snapshot, SNAPSHOT_CONSUMERS>;
pub type SnapshotReceiver = WatchReceiver<'static, CriticalSectionRawMutex, Snapshot, SNAPSHOT_CONSUMERS>;

/* === Structs === */

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Row {
    pub label: Label,

    pub value: Value,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Activity {
    pub request: Value,

    pub response: Value,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalPage {
    pub title: Title,

    pub rows: heapless::Vec<Row, MAX_STATUS_ROWS>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NearbyPage {
    pub id: u32,

    pub title: Title,

    pub rows: heapless::Vec<Row, MAX_STATUS_ROWS>,

    pub activity: Activity,

    pub last_seen_ms: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Snapshot {
    pub local: LocalPage,

    pub activity: Activity,

    pub nearby: heapless::Vec<NearbyPage, MAX_NEARBY_PAGES>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Controls {
    pub previous: bool,

    pub next: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Palette<C> {
    pub background: C,

    pub foreground: C,

    pub muted: C,

    pub accent: C,
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics<C> {
    pub width: usize,

    pub height: usize,

    pub band_height: usize,

    pub title_font: &'static MonoFont<'static>,

    pub body_font: &'static MonoFont<'static>,

    pub palette: Palette<C>,

    pub margin: i32,

    pub header_height: i32,

    pub rows_start: i32,

    pub row_pitch: i32,

    pub footer_height: i32,

    pub separators: bool,

    pub refresh_interval: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenderedPage {
    title: Title,
    badge: &'static str,
    rows: heapless::Vec<Row, MAX_PAGE_ROWS>,
    index: usize,
    count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Navigation {
    page: Page,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PageOrder {
    nearby: [Option<u32>; MAX_NEARBY_PAGES],
    len: usize,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Local,
    Nearby(u32),
}

/* === Traits === */

#[allow(async_fn_in_trait)]
pub trait Panel {
    type Color: PixelColor;

    type Band: DrawTarget<Color = Self::Color, Error = core::convert::Infallible>;

    type Error;

    fn metrics(&self) -> Metrics<Self::Color>;

    fn begin_band(&mut self, y: usize) -> &mut Self::Band;

    async fn present_band(&mut self, y: usize) -> Result<(), Self::Error>;

    async fn sample(&mut self) -> Controls;
}

/* === Implementations === */

impl Snapshot {
    #[must_use]
    pub fn nearby_page(&self, id: u32) -> Option<&NearbyPage> {
        self.nearby.iter().find(|page| page.id == id)
    }
}

impl Navigation {
    const fn new() -> Self {
        Self { page: Page::Local }
    }

    fn page(&mut self, order: &PageOrder) -> Page {
        if order.index(self.page).is_none() {
            self.page = Page::Local;
        }
        self.page
    }

    fn previous(&mut self, order: &PageOrder) {
        let index = order.index(self.page).unwrap_or(0);
        self.page = order.page(index.saturating_sub(1)).unwrap_or(Page::Local);
    }

    fn next(&mut self, order: &PageOrder) {
        let index = order.index(self.page).unwrap_or(0);
        self.page = order.page((index + 1).min(order.len() - 1)).unwrap_or(Page::Local);
    }
}

impl PageOrder {
    fn new(snapshot: &Snapshot) -> Self {
        let mut ordered: [Option<u32>; MAX_NEARBY_PAGES] = [None; MAX_NEARBY_PAGES];
        let mut len = 0;
        for page in &snapshot.nearby {
            let id = page.id;
            let mut insert_at = len;
            while insert_at > 0 && ordered[insert_at - 1].is_some_and(|current| current > id) {
                ordered[insert_at] = ordered[insert_at - 1];
                insert_at -= 1;
            }
            ordered[insert_at] = Some(id);
            len += 1;
        }
        Self { nearby: ordered, len }
    }

    const fn len(&self) -> usize {
        self.len + 1
    }

    fn page(&self, index: usize) -> Option<Page> {
        if index == 0 {
            return Some(Page::Local);
        }
        self.nearby.get(index - 1).and_then(|id| id.map(Page::Nearby))
    }

    fn index(&self, page: Page) -> Option<usize> {
        match page {
            Page::Local => Some(0),
            Page::Nearby(id) => self.nearby.iter().position(|candidate| *candidate == Some(id)).map(|index| index + 1),
        }
    }
}

/* === Public Functions === */

pub async fn run<P: Panel>(mut panel: P, mut snapshots: SnapshotReceiver) -> ! {
    let metrics = panel.metrics();
    let mut snapshot = snapshots.get().await;
    let mut navigation = Navigation::new();
    let mut previous: Option<RenderedPage> = None;
    let mut previous_controls = Controls::default();
    let mut next_refresh = Instant::now();
    loop {
        if let Some(update) = snapshots.try_changed() {
            snapshot = update;
        }
        let order = PageOrder::new(&snapshot);
        let page = navigation.page(&order);
        let now = Instant::now();
        let rendered = compose(&snapshot, &order, page, now.as_millis(), &metrics);
        if now >= next_refresh && previous.as_ref() != Some(&rendered) {
            for band in (0..metrics.height).step_by(metrics.band_height.max(1)) {
                if let Some(previous) = &previous
                    && !band_dirty(&rendered, previous, band, &metrics)
                {
                    continue;
                }
                let target = panel.begin_band(band);
                draw_page(target, &metrics, &rendered);
                if panel.present_band(band).await.is_err() {
                    defmt::error!("display band flush failed; display engine parked");
                    core::future::pending::<()>().await;
                }
            }
            previous = Some(rendered);
            next_refresh = now + metrics.refresh_interval;
        }
        let controls = panel.sample().await;
        if controls.previous && !previous_controls.previous {
            navigation.previous(&order);
        }
        if controls.next && !previous_controls.next {
            navigation.next(&order);
        }
        previous_controls = controls;
        Timer::after(INPUT_POLL).await;
    }
}

/* === Private Functions === */

fn compose<C: PixelColor>(snapshot: &Snapshot, order: &PageOrder, page: Page, now_ms: u64, metrics: &Metrics<C>) -> RenderedPage {
    let index = order.index(page).unwrap_or(0);
    let count = order.len();
    let budget = visible_rows(metrics);
    if let Page::Nearby(id) = page
        && let Some(nearby) = snapshot.nearby_page(id)
    {
        let mut rows: heapless::Vec<Row, MAX_PAGE_ROWS> = heapless::Vec::new();
        let _ = rows.push(Row { label: text("Last Seen"), value: age_value(now_ms, nearby.last_seen_ms) });
        for row in &nearby.rows {
            let _ = rows.push(row.clone());
        }
        let _ = rows.push(Row { label: text("Last Request"), value: nearby.activity.request.clone() });
        let _ = rows.push(Row { label: text("Last Response"), value: nearby.activity.response.clone() });
        // Rows are pushed in the order they should win when the panel's
        // geometry cannot show them all; the row list is built in full
        // above (never silently dropped by `Vec` capacity) and only then
        // cut to what `metrics` can actually render.
        rows.truncate(budget);
        return RenderedPage { title: nearby.title.clone(), badge: NEARBY_BADGE, rows, index, count };
    }
    let mut rows: heapless::Vec<Row, MAX_PAGE_ROWS> = heapless::Vec::new();
    let _ = rows.push(Row { label: text("Last Request"), value: snapshot.activity.request.clone() });
    let _ = rows.push(Row { label: text("Last Response"), value: snapshot.activity.response.clone() });
    for row in &snapshot.local.rows {
        let _ = rows.push(row.clone());
    }
    // The local page pushes activity before status rows, so the same
    // tail-truncation keeps activity and lets the panel's row budget bound
    // how many trailing status rows are shown.
    rows.truncate(budget);
    RenderedPage { title: snapshot.local.title.clone(), badge: LOCAL_BADGE, rows, index, count }
}

fn visible_rows<C: PixelColor>(metrics: &Metrics<C>) -> usize {
    let footer_top = coordinate(metrics.height) - metrics.footer_height;
    let available = (footer_top - metrics.rows_start).max(0);
    if metrics.row_pitch <= 0 {
        return 0;
    }
    usize::try_from(available / metrics.row_pitch).unwrap_or(0)
}

fn band_dirty<C: PixelColor>(current: &RenderedPage, previous: &RenderedPage, band_y: usize, metrics: &Metrics<C>) -> bool {
    let band_height = metrics.band_height.max(1);
    let overlaps = |region_y: i32, region_height: i32| {
        let band_y = coordinate(band_y);
        let band_height = coordinate(band_height);
        band_y < region_y + region_height && band_y + band_height > region_y
    };
    if overlaps(0, metrics.header_height) && (current.title != previous.title || current.badge != previous.badge) {
        return true;
    }
    let rows = current.rows.len().max(previous.rows.len());
    for index in 0..rows {
        if current.rows.get(index) != previous.rows.get(index)
            && overlaps(metrics.rows_start + coordinate(index) * metrics.row_pitch, metrics.row_pitch)
        {
            return true;
        }
    }
    (current.index != previous.index || current.count != previous.count)
        && overlaps(coordinate(metrics.height) - metrics.footer_height, metrics.footer_height)
}

fn draw_page<D: DrawTarget>(target: &mut D, metrics: &Metrics<D::Color>, page: &RenderedPage) {
    let _ = target.clear(metrics.palette.background);
    let top = TextStyleBuilder::new().baseline(Baseline::Top).build();
    let width = coordinate(metrics.width);
    let height = coordinate(metrics.height);

    // Header: title, badge, optional separator.
    let title_style = MonoTextStyle::new(metrics.title_font, metrics.palette.foreground);
    let title_y = (metrics.header_height - font_height(metrics.title_font)).max(0) / 2;
    let _ = Text::with_text_style(&page.title, Point::new(metrics.margin, title_y), title_style, top).draw(target);
    let badge_style = MonoTextStyle::new(metrics.body_font, metrics.palette.muted);
    let badge_y = (metrics.header_height - font_height(metrics.body_font)).max(0) / 2;
    let badge_x = width - metrics.margin - text_width(metrics.body_font, page.badge.len());
    let _ = Text::with_text_style(page.badge, Point::new(badge_x, badge_y), badge_style, top).draw(target);
    if metrics.separators {
        let y = metrics.header_height - 2;
        let _ = Line::new(Point::new(metrics.margin, y), Point::new(width - metrics.margin - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(metrics.palette.muted, 1))
            .draw(target);
    }

    // Rows: label left, value right.
    let label_style = MonoTextStyle::new(metrics.body_font, metrics.palette.muted);
    let value_style = MonoTextStyle::new(metrics.body_font, metrics.palette.foreground);
    for (index, row) in page.rows.iter().enumerate() {
        let y = metrics.rows_start + coordinate(index) * metrics.row_pitch;
        let _ = Text::with_text_style(&row.label, Point::new(metrics.margin, y), label_style, top).draw(target);
        let value_x = width - metrics.margin - text_width(metrics.body_font, row.value.len());
        let _ = Text::with_text_style(&row.value, Point::new(value_x, y), value_style, top).draw(target);
    }

    // Footer: navigation arrows and the page indicator.
    let footer_top = height - metrics.footer_height;
    if metrics.separators {
        let _ = Line::new(Point::new(metrics.margin, footer_top), Point::new(width - metrics.margin - 1, footer_top))
            .into_styled(PrimitiveStyle::with_stroke(metrics.palette.muted, 1))
            .draw(target);
    }
    let previous_color = if page.index > 0 { metrics.palette.accent } else { metrics.palette.muted };
    let next_color = if page.index + 1 < page.count { metrics.palette.accent } else { metrics.palette.muted };
    let arrow_y = footer_top + (metrics.footer_height - font_height(metrics.title_font)).max(0) / 2;
    let left_style = MonoTextStyle::new(metrics.title_font, previous_color);
    let right_style = MonoTextStyle::new(metrics.title_font, next_color);
    let _ = Text::with_text_style("<", Point::new(metrics.margin, arrow_y), left_style, top).draw(target);
    let arrow_x = width - metrics.margin - text_width(metrics.title_font, 1);
    let _ = Text::with_text_style(">", Point::new(arrow_x, arrow_y), right_style, top).draw(target);
    let mut indicator = heapless::String::<16>::new();
    let _ = write!(indicator, "{} / {}", page.index + 1, page.count);
    let indicator_x = (width - text_width(metrics.body_font, indicator.len())) / 2;
    let indicator_y = footer_top + (metrics.footer_height - font_height(metrics.body_font)).max(0) / 2;
    let indicator_style = MonoTextStyle::new(metrics.body_font, metrics.palette.foreground);
    let _ = Text::with_text_style(&indicator, Point::new(indicator_x, indicator_y), indicator_style, top).draw(target);
}

fn age_value(now_ms: u64, last_seen_ms: u64) -> Value {
    let mut value = Value::new();
    let age_ms = now_ms.saturating_sub(last_seen_ms);
    if age_ms < 1_000 {
        let _ = value.push_str("<1 s");
    } else {
        let _ = write!(value, "{} s", age_ms / 1_000);
    }
    value
}

fn text<const N: usize>(value: &str) -> heapless::String<N> {
    let mut out = heapless::String::new();
    let _ = out.push_str(value);
    out
}

fn font_height(font: &MonoFont<'_>) -> i32 {
    i32::try_from(font.character_size.height).unwrap_or(i32::MAX)
}

fn text_width(font: &MonoFont<'_>, characters: usize) -> i32 {
    let advance = font.character_size.width + font.character_spacing;
    coordinate(characters) * i32::try_from(advance).unwrap_or(i32::MAX)
}

fn coordinate(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    fn nearby(id: u32) -> NearbyPage {
        NearbyPage { id, title: text("Peer"), rows: heapless::Vec::new(), activity: Activity::default(), last_seen_ms: 0 }
    }

    fn snapshot_with_nearby() -> Snapshot {
        let mut snapshot = Snapshot {
            local: LocalPage { title: text("Local"), rows: heapless::Vec::new() },
            activity: Activity::default(),
            nearby: heapless::Vec::new(),
        };
        let _ = snapshot.nearby.push(nearby(200_000));
        snapshot
    }

    fn tft_like_metrics() -> Metrics<embedded_graphics::pixelcolor::BinaryColor> {
        use embedded_graphics::pixelcolor::BinaryColor;
        Metrics {
            width: 320,
            height: 240,
            band_height: 10,
            title_font: &embedded_graphics::mono_font::ascii::FONT_10X20,
            body_font: &embedded_graphics::mono_font::ascii::FONT_6X10,
            palette: Palette { background: BinaryColor::Off, foreground: BinaryColor::On, muted: BinaryColor::On, accent: BinaryColor::On },
            margin: 10,
            header_height: 40,
            rows_start: 48,
            row_pitch: 24,
            footer_height: 46,
            separators: true,
            refresh_interval: Duration::from_secs(1),
        }
    }

    fn oled_like_metrics() -> Metrics<embedded_graphics::pixelcolor::BinaryColor> {
        use embedded_graphics::pixelcolor::BinaryColor;
        Metrics {
            width: 128,
            height: 64,
            band_height: 64,
            title_font: &embedded_graphics::mono_font::ascii::FONT_5X7,
            body_font: &embedded_graphics::mono_font::ascii::FONT_5X7,
            palette: Palette { background: BinaryColor::Off, foreground: BinaryColor::On, muted: BinaryColor::On, accent: BinaryColor::On },
            margin: 1,
            header_height: 8,
            rows_start: 8,
            row_pitch: 8,
            footer_height: 8,
            separators: false,
            refresh_interval: Duration::from_secs(1),
        }
    }

    #[test]
    fn arrows_wrap() {
        let order = PageOrder::new(&snapshot_with_nearby());
        let mut navigation = Navigation::new();
        navigation.previous(&order);
        assert_eq!(navigation.page(&order), Page::Local);
        navigation.next(&order);
        navigation.next(&order);
        assert_eq!(navigation.page(&order), Page::Nearby(200_000));
    }

    #[test]
    fn disappearing_peer_page() {
        let mut navigation = Navigation::new();
        navigation.next(&PageOrder::new(&snapshot_with_nearby()));
        let empty = PageOrder::new(&Snapshot::default());
        assert_eq!(navigation.page(&empty), Page::Local);
    }

    #[test]
    fn pages_include_order() {
        let mut snapshot = Snapshot::default();
        for offset in (0..MAX_NEARBY_PAGES).rev() {
            let _ = snapshot.nearby.push(nearby(200_000 + u32::try_from(offset).unwrap()));
        }
        let order = PageOrder::new(&snapshot);
        assert_eq!(order.len(), MAX_NEARBY_PAGES + 1);
        assert_eq!(order.page(0), Some(Page::Local));
        assert_eq!(order.page(1), Some(Page::Nearby(200_000)));
        assert_eq!(order.page(MAX_NEARBY_PAGES), Some(Page::Nearby(200_000 + u32::try_from(MAX_NEARBY_PAGES - 1).unwrap())));
    }

    #[test]
    fn local_page_rows() {
        let mut snapshot = snapshot_with_nearby();
        let _ = snapshot.local.rows.push(Row { label: text("Channels"), value: text("1:Free") });
        let order = PageOrder::new(&snapshot);
        let rendered = compose(&snapshot, &order, Page::Local, 0, &tft_like_metrics());
        assert_eq!(rendered.badge, "Ready");
        assert_eq!(rendered.rows[0].label.as_str(), "Last Request");
        assert_eq!(rendered.rows[1].label.as_str(), "Last Response");
        assert_eq!(rendered.rows[2].label.as_str(), "Channels");
        assert_eq!(rendered.index, 0);
        assert_eq!(rendered.count, 2);
    }

    #[test]
    fn row_value_bands() {
        let mut snapshot = snapshot_with_nearby();
        let _ = snapshot.local.rows.push(Row { label: text("RTT Last / Mean"), value: text("N/A") });
        let order = PageOrder::new(&snapshot);
        let metrics = tft_like_metrics();
        let previous = compose(&snapshot, &order, Page::Local, 0, &metrics);
        let mut current = previous.clone();
        current.rows[2].value = text("86 / 87 ms");

        let dirty: heapless::Vec<usize, 24> =
            (0..metrics.height).step_by(metrics.band_height).filter(|band| band_dirty(&current, &previous, *band, &metrics)).collect();

        // Row 2 occupies the region starting at 48 + 2 * 24 = 96.
        assert_eq!(dirty.as_slice(), &[90, 100, 110]);
    }

    #[test]
    fn both_shipped_rows() {
        assert_eq!(visible_rows(&tft_like_metrics()), 6);
        assert_eq!(visible_rows(&oled_like_metrics()), 6);
    }

    #[test]
    fn local_page_budget() {
        const LABELS: [&str; MAX_STATUS_ROWS] = ["Row0", "Row1", "Row2", "Row3", "Row4", "Row5", "Row6"];
        let mut snapshot = snapshot_with_nearby();
        // Fill the driver model's full lossless capacity (`MAX_STATUS_ROWS`
        // status rows, e.g. every chassis row plus a synthesized `Channels`
        // row) so the panel's physical budget, not the projection, is what
        // decides which rows are shown.
        for label in LABELS {
            let _ = snapshot.local.rows.push(Row { label: text(label), value: text("x") });
        }
        assert_eq!(snapshot.local.rows.len(), MAX_STATUS_ROWS);

        let order = PageOrder::new(&snapshot);
        let metrics = tft_like_metrics();
        let budget = visible_rows(&metrics);
        let rendered = compose(&snapshot, &order, Page::Local, 0, &metrics);

        assert_eq!(rendered.rows.len(), budget);
        assert_eq!(rendered.rows[0].label.as_str(), "Last Request");
        assert_eq!(rendered.rows[1].label.as_str(), "Last Response");
        // Status rows fill the remaining budget in push order (earliest
        // status rows win); the trailing ones are the documented drop.
        for (offset, row) in rendered.rows[2..].iter().enumerate() {
            assert_eq!(row.label.as_str(), LABELS[offset]);
        }
    }
}
