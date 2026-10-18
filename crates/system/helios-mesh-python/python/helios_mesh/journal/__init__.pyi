# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

# === Imports === #

# Standard Library Imports
from typing import final

# Third Party Imports
# Local Imports
from helios_mesh.node import JournalEntry, JournalEvent, NodeHandle, NodeId

# Type Checking Imports

# === Public Exports === #

__all__ = [
    "RECORD_VERSION",
    "SCHEMA_VERSION",
    "Cursor",
    "Header",
    "Metadata",
    "Record",
    "event_name",
    "now_us",
]

# === Constants === #

SCHEMA_VERSION: int

RECORD_VERSION: int

# === Types === #

# === Classes === #

@final
class Header:
    def __new__(cls, node: NodeId) -> Header: ...
    @property
    def schema(self) -> int: ...
    @property
    def node(self) -> int: ...
    @property
    def captured_at_us(self) -> int: ...
    def to_json(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Metadata:
    @property
    def node(self) -> int: ...
    @property
    def device_sequence(self) -> int: ...
    @property
    def device_uptime_ms(self) -> int: ...
    @property
    def observed_at_us(self) -> int: ...
    def __repr__(self) -> str: ...

@final
class Record:
    def __new__(cls, position: int, node: NodeId, entry: JournalEntry) -> Record: ...
    @property
    def position(self) -> int: ...
    @property
    def kind(self) -> str: ...
    @property
    def id(self) -> str: ...
    @property
    def version(self) -> int: ...
    @property
    def metadata(self) -> Metadata: ...
    @property
    def payload(self) -> JournalEvent: ...
    def to_json(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Cursor:
    def __new__(cls) -> Cursor: ...
    @property
    def position(self) -> int: ...
    @property
    def latest(self) -> int: ...
    @property
    def is_caught_up(self) -> bool: ...
    async def next(self, node: NodeHandle) -> list[JournalEntry]: ...
    def __repr__(self) -> str: ...

# === Public Functions === #

def event_name(event: JournalEvent) -> str: ...
def now_us() -> int: ...

# === Private Functions === #

# === Entry Point === #
