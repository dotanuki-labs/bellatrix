// Copyright 2026 Dotanuki Labs
// SPDX-License-Identifier: AGPL-3.0-or-later

use worker::*;

#[event(scheduled)]
async fn scheduled(_req: ScheduledEvent, _env: Env, _ctx: ScheduleContext) {
    console_log!("Scheduling task finished");
}
