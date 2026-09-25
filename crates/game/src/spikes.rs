//! `cargo run -p platypus --release --features spikes`: times every Bevy
//! system (through `bevy/trace`'s spans, render world included) and logs the
//! heaviest of any frame slower than `PLATYPUS_SPIKES` ms (default 12).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bevy::log::BoxedLayer;
use bevy::prelude::*;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

/// Time per span name (and calls) since the last frame report.
static TIMES: Mutex<Option<HashMap<String, (Duration, u32)>>> = Mutex::new(None);

struct SpanName(String);
struct Entered(Instant);

struct Timing;

struct NameField(Option<String>);

impl Visit for NameField {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "name" {
            self.0 = Some(value.to_string());
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.0 = Some(format!("{value:?}").trim_matches('"').to_string());
        }
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Timing {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut name = NameField(None);
        attrs.record(&mut name);
        let name = name.0.unwrap_or_else(|| attrs.metadata().name().to_string());
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanName(name));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().replace(Entered(Instant::now()));
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut ext = span.extensions_mut();
        let Some(Entered(at)) = ext.remove::<Entered>() else { return };
        let Some(SpanName(name)) = ext.get_mut::<SpanName>() else { return };
        let mut times = TIMES.lock().unwrap();
        let e = times.get_or_insert_with(HashMap::new).entry(name.clone()).or_default();
        e.0 += at.elapsed();
        e.1 += 1;
    }
}

pub fn layer(_app: &mut App) -> Option<BoxedLayer> {
    Some(Box::new(Timing))
}

pub struct SpikesPlugin;

impl Plugin for SpikesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, report);
    }
}

fn report(time: Res<Time<Real>>) {
    let limit = std::env::var("PLATYPUS_SPIKES").ok().and_then(|s| s.parse().ok()).unwrap_or(12.0);
    let Some(times) = TIMES.lock().unwrap().take() else { return };
    let dt = time.delta().as_secs_f32() * 1000.0;
    // Our own work (the sim, streaming, drawing prep), apart from waiting on
    // the GPU: over half a 120 Hz frame is a spike too.
    let ours: f32 = times.iter().filter(|(n, _)| n.starts_with("platypus::")).map(|(_, (d, _))| d.as_secs_f32() * 1000.0).sum();
    if dt < limit && ours < limit / 2.0 {
        return;
    }
    // Leaf spans only: schedules and apps contain the systems.
    const CONTAINERS: [&str; 16] = ["update", "main app", "Main", "RenderApp", "RenderRecovery", "Render", "RenderExtractApp", "PostUpdate", "PreUpdate", "Update", "RunFixedMainLoop", "FixedMain", "FixedUpdate", "ExtractSchedule", "multithreaded executor", "RenderGraph"];
    let mut top: Vec<_> = times.into_iter().filter(|(n, _)| !CONTAINERS.contains(&n.as_str())).collect();
    top.sort_by_key(|(_, (d, _))| std::cmp::Reverse(*d));
    let list: Vec<String> = top.iter().take(10).map(|(n, (d, c))| format!("{:.1} {n}{}", d.as_secs_f32() * 1000.0, if *c > 1 { format!(" ×{c}") } else { String::new() })).collect();
    info!("spike {dt:.1} ms (ours {ours:.1}): {}", list.join(" | "));
}
