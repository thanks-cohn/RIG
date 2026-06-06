use proc_macro::TokenStream;

#[proc_macro_derive(Serialize)]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    if input.contains("struct ArenaTotals") {
        quote_arena_totals_serialize().parse().unwrap()
    } else if input.contains("struct ContainerReport") {
        quote_container_report_serialize().parse().unwrap()
    } else if input.contains("struct ArenaReport") {
        quote_arena_report_serialize().parse().unwrap()
    } else {
        TokenStream::new()
    }
}

#[proc_macro_derive(Deserialize)]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    if input.contains("struct ArenaTotals") {
        quote_arena_totals_deserialize().parse().unwrap()
    } else if input.contains("struct ContainerReport") {
        quote_container_report_deserialize().parse().unwrap()
    } else if input.contains("struct ArenaReport") {
        quote_arena_report_deserialize().parse().unwrap()
    } else {
        TokenStream::new()
    }
}

fn quote_arena_totals_serialize() -> &'static str {
    r#"
impl serde::Serialize for ArenaTotals {
    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let inner = " ".repeat(indent + 2);
        format!(
            "{{\n{inner}\"total_len\": {},\n{inner}\"total_current_capacity\": {},\n{inner}\"total_growth_events\": {},\n{inner}\"total_pushed_appended_operations\": {}\n{pad}}}",
            self.total_len,
            self.total_current_capacity,
            self.total_growth_events,
            self.total_pushed_appended_operations,
            inner = inner,
            pad = pad,
        )
    }
}
"#
}

fn quote_container_report_serialize() -> &'static str {
    r#"
impl serde::Serialize for ContainerReport {
    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let inner = " ".repeat(indent + 2);
        let extra_label = match &self.extra_metric_label {
            Some(value) => format!("\"{}\"", serde_json::escape_string(value)),
            None => "null".to_string(),
        };
        format!(
            "{{\n{inner}\"name\": \"{}\",\n{inner}\"kind\": \"{}\",\n{inner}\"len\": {},\n{inner}\"initial_capacity\": {},\n{inner}\"current_capacity\": {},\n{inner}\"growth_events\": {},\n{inner}\"operation_label\": \"{}\",\n{inner}\"total_operations\": {},\n{inner}\"extra_metric_label\": {},\n{inner}\"extra_metric_value\": {}\n{pad}}}",
            serde_json::escape_string(&self.name),
            serde_json::escape_string(&self.kind),
            self.len,
            self.initial_capacity,
            self.current_capacity,
            self.growth_events,
            serde_json::escape_string(&self.operation_label),
            self.total_operations,
            extra_label,
            self.extra_metric_value,
            inner = inner,
            pad = pad,
        )
    }
}
"#
}

fn quote_arena_report_serialize() -> &'static str {
    r#"
impl serde::Serialize for ArenaReport {
    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let inner = " ".repeat(indent + 2);
        let container_indent = indent + 4;
        let containers = if self.containers.is_empty() {
            String::new()
        } else {
            let rendered: Vec<String> = self
                .containers
                .iter()
                .map(|container| format!("{}{}", " ".repeat(container_indent), container.to_json_pretty(container_indent)))
                .collect();
            format!("\n{}\n{}", rendered.join(",\n"), inner)
        };
        format!(
            "{{\n{inner}\"arena_name\": \"{}\",\n{inner}\"tracked_container_count\": {},\n{inner}\"totals\": {},\n{inner}\"containers\": [{}]\n{pad}}}",
            serde_json::escape_string(&self.arena_name),
            self.tracked_container_count,
            self.totals.to_json_pretty(indent + 2),
            containers,
            inner = inner,
            pad = pad,
        )
    }
}
"#
}

fn quote_arena_totals_deserialize() -> &'static str {
    r#"
impl serde::Deserialize for ArenaTotals {
    fn from_json_str(input: &str) -> Result<Self, String> {
        let value = serde_json::Value::parse(input)?;
        Ok(Self {
            total_len: value.field_usize("total_len")?,
            total_current_capacity: value.field_usize("total_current_capacity")?,
            total_growth_events: value.field_usize("total_growth_events")?,
            total_pushed_appended_operations: value.field_usize("total_pushed_appended_operations")?,
        })
    }
}
"#
}

fn quote_container_report_deserialize() -> &'static str {
    r#"
impl serde::Deserialize for ContainerReport {
    fn from_json_str(input: &str) -> Result<Self, String> {
        let value = serde_json::Value::parse(input)?;
        Ok(Self {
            name: value.field_string("name")?,
            kind: value.field_string("kind")?,
            len: value.field_usize("len")?,
            initial_capacity: value.field_usize("initial_capacity")?,
            current_capacity: value.field_usize("current_capacity")?,
            growth_events: value.field_usize("growth_events")?,
            operation_label: value.field_string("operation_label")?,
            total_operations: value.field_usize("total_operations")?,
            extra_metric_label: value.field_option_string("extra_metric_label")?,
            extra_metric_value: value.field_usize("extra_metric_value")?,
        })
    }
}
"#
}

fn quote_arena_report_deserialize() -> &'static str {
    r#"
impl serde::Deserialize for ArenaReport {
    fn from_json_str(input: &str) -> Result<Self, String> {
        let value = serde_json::Value::parse(input)?;
        let totals = ArenaTotals::from_json_str(&value.field_value("totals")?.to_compact_string())?;
        let containers = value
            .field_array("containers")?
            .iter()
            .map(|container| ContainerReport::from_json_str(&container.to_compact_string()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            arena_name: value.field_string("arena_name")?,
            tracked_container_count: value.field_usize("tracked_container_count")?,
            totals,
            containers,
        })
    }
}
"#
}
