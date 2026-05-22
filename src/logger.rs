use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
    level_filters::LevelFilter,
};
use tracing_subscriber::{
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    registry::LookupSpan,
};

pub struct SuperoxideVisitor<'a, 'b> {
    writer: &'a mut Writer<'b>,
}

impl<'a, 'b> Visit for SuperoxideVisitor<'a, 'b> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            write!(self.writer, "{value:?}").unwrap();
        }
    }
}

struct SuperoxideFormatEvent;

impl<S, N> FormatEvent<S, N> for SuperoxideFormatEvent
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let local = chrono::Local::now();

        // x1B[90m - light gray
        write!(
            writer,
            "{}",
            local.format("\x1B[90m%d/%m/%Y %H:%M:%S \x1B[0m")
        )?;

        let level = event.metadata().level();
        let header = match *level {
            Level::ERROR => "\x1B[1;97;101mERROR",
            Level::WARN => " \x1B[1;30;103mWARN",
            Level::INFO => " \x1B[97;104mINFO",
            Level::DEBUG => "\x1B[30;107mDEBUG",
            Level::TRACE => "\x1B[97;100mTRACE",
        };

        let text_color = match *level {
            Level::ERROR => "\x1B[91m",
            Level::WARN => "\x1B[93m",
            Level::INFO => "\x1B[94m",
            Level::TRACE => "\x1B[90m",
            _ => "",
        };

        write!(writer, "{header}\x1B[0m {text_color}")?;
        event.record(&mut SuperoxideVisitor {
            writer: &mut writer,
        });
        writeln!(writer, "\x1B[0m")?;

        Ok(())
    }
}

pub fn init() {
    tracing_subscriber::fmt()
        .event_format(SuperoxideFormatEvent)
        .with_max_level(LevelFilter::from_level(Level::DEBUG))
        .init();
}
