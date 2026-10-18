/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Projection from chassis UI snapshots onto the driver display model.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::fmt::Write as _;

// Third Party Imports

// Local Imports
use hnm_core::{
    node::StatusValue,
    ui::{ActivityDirection, ActivityState, Receiver, RequestActivity, Snapshot},
};
use hnm_drivers::display;
use hnm_protocol::node::NodeId;

/* === Constants === */

// This is the only module that sees both the chassis's status-row budget and
// the driver's row capacity, so the tie between them lives here rather than
// as prose.
const _: () = assert!(display::MAX_STATUS_ROWS >= hnm_core::node::MAX_STATUS_ROWS, "driver row capacity must fit every chassis status row");

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub async fn run(mut snapshots: Receiver, watch: &'static display::SnapshotWatch) -> ! {
    let sender = watch.sender();
    let mut snapshot = snapshots.get().await;
    loop {
        sender.send(project(&snapshot));
        snapshot = snapshots.changed().await;
    }
}

#[must_use]
pub fn project(snapshot: &Snapshot) -> display::Snapshot {
    let local_id = snapshot.local.id;
    let mut local = display::LocalPage { title: title(snapshot.local.label, local_id), rows: heapless::Vec::new() };
    for row in snapshot.local.rows.rows() {
        push_row(&mut local.rows, row.label, status_value(row.value));
    }

    let mut view = display::Snapshot { local, activity: activity(snapshot.activity, local_id, local_id), nearby: heapless::Vec::new() };
    for page in most_recently_seen(snapshot) {
        let _ = view.nearby.push(display::NearbyPage {
            id: page.id.0,
            title: title(page.label.as_str(), page.id),
            rows: heapless::Vec::new(),
            activity: activity(snapshot.nearby_activity(page.id), page.id, local_id),
            last_seen_ms: page.last_seen_ms,
        });
    }
    view
}

/* === Private Functions === */

fn most_recently_seen(snapshot: &Snapshot) -> heapless::Vec<&hnm_core::ui::NearbyPage, { display::MAX_NEARBY_PAGES }> {
    let mut selected: heapless::Vec<&hnm_core::ui::NearbyPage, { display::MAX_NEARBY_PAGES }> = heapless::Vec::new();
    for page in &snapshot.nearby {
        let position = selected.iter().position(|kept| page.last_seen_ms > kept.last_seen_ms).unwrap_or(selected.len());
        if selected.is_full() {
            if position == selected.len() {
                continue;
            }
            let _ = selected.pop();
        }
        let _ = selected.insert(position, page);
    }
    selected
}

fn title(label: &str, id: NodeId) -> display::Title {
    let mut title = display::Title::new();
    if label.is_empty() {
        let _ = write!(title, "{:06}", id.0);
    } else {
        let _ = write!(title, "{label} {:06}", id.0);
    }
    title
}

fn push_row(rows: &mut heapless::Vec<display::Row, { display::MAX_STATUS_ROWS }>, label: &str, value: display::Value) {
    let mut row = display::Row { label: display::Label::new(), value };
    let _ = row.label.push_str(label);
    let _ = rows.push(row);
}

fn status_value(value: StatusValue) -> display::Value {
    let mut text = display::Value::new();
    match value {
        StatusValue::Count(count) => {
            let _ = write!(text, "{count}");
        }
        StatusValue::Ratio(count, total) => {
            let _ = write!(text, "{count} / {total}");
        }
        StatusValue::RoundTrip(stats) => {
            let _ = write!(text, "{} / {} ms", stats.last_ms, stats.mean_ms);
        }
        StatusValue::Text(value) => {
            let _ = text.push_str(value);
        }
    }
    text
}

fn activity(activity: Option<RequestActivity>, viewpoint: NodeId, observer: NodeId) -> display::Activity {
    display::Activity { request: request_value(activity, viewpoint, observer), response: response_value(activity) }
}

fn request_value(activity: Option<RequestActivity>, viewpoint: NodeId, observer: NodeId) -> display::Value {
    let mut value = display::Value::new();
    match activity.and_then(|activity| activity.direction_from(viewpoint, observer)) {
        Some(ActivityDirection::Outgoing(node)) => {
            let _ = write!(value, "To {:06}", node.0);
        }
        Some(ActivityDirection::Incoming(node)) => {
            let _ = write!(value, "From {:06}", node.0);
        }
        Some(ActivityDirection::LocalUsb) => {
            let _ = value.push_str("Local USB");
        }
        None => {
            let _ = value.push_str("None Yet");
        }
    }
    value
}

fn response_value(activity: Option<RequestActivity>) -> display::Value {
    let mut value = display::Value::new();
    let _ = value.push_str(match activity.map(|activity| activity.state) {
        Some(ActivityState::Pending) => "Pending",
        Some(ActivityState::Succeeded) => "Succeeded",
        Some(ActivityState::Failed) => "Failed",
        None => "None Yet",
    });
    value
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_core::node::Origin;
    use hnm_core::ui::{LocalPage, NearbyPage};
    use hnm_protocol::node::{NodeLabel, RequestId};

    use super::*;

    const LOCAL: NodeId = NodeId(566_114);
    const PEER: NodeId = NodeId(273_282);

    fn nearby(id: u32, last_seen_ms: u64) -> NearbyPage {
        NearbyPage { id: NodeId(id), label: NodeLabel::try_from("rotator").unwrap(), last_seen_ms }
    }

    #[test]
    fn local_page_activity() {
        let mut snapshot = Snapshot::new(LocalPage::new(LOCAL, "rotator"));
        snapshot.local.rows.push("Nearby Devices", StatusValue::Count(3));
        snapshot.activity = Some(RequestActivity::observed(Origin::Usb, RequestId(17), LOCAL));

        let view = project(&snapshot);
        assert_eq!(view.local.title.as_str(), "rotator 566114");
        assert_eq!(view.local.rows[0].label.as_str(), "Nearby Devices");
        assert_eq!(view.local.rows[0].value.as_str(), "3");
        assert_eq!(view.activity.request.as_str(), "Local USB");
        assert_eq!(view.activity.response.as_str(), "Pending");
    }

    #[test]
    fn forwarded_activity_page() {
        let mut snapshot = Snapshot::new(LocalPage::new(LOCAL, "gateway"));
        snapshot.upsert_nearby(nearby(PEER.0, 5));
        snapshot.activity = Some(RequestActivity::observed(Origin::Usb, RequestId(20), PEER));

        let view = project(&snapshot);
        assert_eq!(view.activity.request.as_str(), "To 273282");
        assert_eq!(view.nearby[0].title.as_str(), "rotator 273282");
        assert_eq!(view.nearby[0].activity.request.as_str(), "From 566114");
    }

    #[test]
    fn nearby_pages_devices() {
        let mut snapshot = Snapshot::new(LocalPage::new(LOCAL, "gateway"));
        for offset in 0..=display::MAX_NEARBY_PAGES {
            let offset = u32::try_from(offset).unwrap();
            snapshot.upsert_nearby(nearby(200_000 + offset, u64::from(offset)));
        }

        let view = project(&snapshot);
        assert_eq!(view.nearby.len(), display::MAX_NEARBY_PAGES);
        // The oldest sighting (offset zero) is the one evicted.
        assert!(view.nearby.iter().all(|page| page.id != 200_000));
        assert_eq!(view.nearby[0].id, 200_008);
    }
}
