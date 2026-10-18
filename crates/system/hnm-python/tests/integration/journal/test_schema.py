# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Journal record schema versions and cursor pagination state."""

# === Imports === #

# Standard Library Imports
import json

# Third Party Imports
# Local Imports
from hnm.journal import RECORD_VERSION, SCHEMA_VERSION, Cursor, Header
from hnm.node import JournalEvent, NodeId

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def test_header_serializes_schema() -> None:
    header = Header(NodeId(641905))

    line = json.loads(header.to_json())

    assert line["schema"] == SCHEMA_VERSION
    assert header.schema == SCHEMA_VERSION
    assert line["node"] == 641905
    assert line["captured_at_us"] == header.captured_at_us
    assert RECORD_VERSION == 1


def test_cursor_starts_entry() -> None:
    cursor = Cursor()

    assert cursor.position == 0
    assert cursor.latest == 0
    assert not cursor.is_caught_up


def test_i2c_baud_neighbor() -> None:
    # hcnt and lcnt are deliberately distinct so a transposed match arm in
    # the Python binding's getter (crates/system/hnm-python/src/node.rs) fails
    # this assertion instead of passing by coincidence.
    event = JournalEvent.i2c_baud_applied(frequency_hz=400_000, hcnt=126, lcnt=187, spklen=11)

    assert event.name == "hnm.i2c.baud-applied"
    assert event.frequency_hz == 400_000
    assert event.hcnt == 126
    assert event.lcnt == 187
    assert event.spklen == 11
    assert event.peer is None
    assert event.request_id is None
    assert event.channel is None
    assert event.operation is None
    assert event.requested is None
    assert event.actual is None
    assert event.timing is None


# === Private Functions === #

# === Entry Point === #
