# Repository Ownership And Connection Deadlines

## Decisions

Local authoring metadata lives in `.stkr/project.toml`. Project-owned runtime policy lives in `tools/project.toml`; native GitHub jobs own routing and artifact dependencies. Public CI needs no private tooling.

Ordinary dev pushes run checks and Product Build. Release-preparation commits also run the exact required native checks retained by existing main protection. Existing dev protection requires `CI / Required`; retain that protection and squash as soon as GitHub permits. No protection is removed or bypassed.

Nightly builds release-profile products and packages independently of suites. Per-job supersession cancels obsolete validation; the publisher has a separate noncancelling concurrency identity. A final job records suite results only if the published channel still identifies that run's commit.

The next version is 0.1.5: the public API, typed errors and protocol 14 remain compatible. Deadline repair restores bounded transport behavior rather than changing operation policy.

## Connection Findings

An awaited USB write prevented the driver's expiry loop from running. A caller also had no queueing deadline. One request deadline now bounds queue admission, writing and response waiting. Expired queued requests are discarded; a partially written frame closes the driver without replaying GPIO or motion commands.

USB opening already retries exclusive-open races five times with a 100 ms pause. Preserve that policy. Port errors now name the port. Failed gateway probes were discarded; tracing now retains their error and port, while request failures retain node, route and request type. Applications own subscriber installation and capturing those messages.

Existing AUX GPIO validation rejects invalid pins, durations, reservation conflicts and busy resources with typed errors. No RF, electrical or downstream-plugin cause is established by the supplied backtraces. Hardware writes are not retried after uncertain acknowledgements.

The local authoring checker required two repairs in its separate fixes checkout: allow native automation tests and normalize CRLF templates before parsing header/documentation boundaries. These repairs are outside the product repository.
