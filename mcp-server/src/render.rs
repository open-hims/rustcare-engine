//! Response rendering utilities for MCP tools
//!
//! Provides utilities to render tool responses in different formats
//! based on the render_type specified in the tool definition.

use crate::protocol::{RenderType, ToolResult, ResponseType};
use serde_json::Value;

/// Render a tool result according to its render type
pub fn render_result(result: &ToolResult, render_type: Option<&RenderType>) -> String {
    let render_type = render_type
        .or_else(|| result.response_type.as_ref().and_then(|rt| rt.render_type.as_ref()))
        .unwrap_or(&RenderType::Json);
    
    match render_type {
        RenderType::Json => render_json(result),
        RenderType::Markdown => render_markdown(result),
        RenderType::Html => render_html(result),
        RenderType::Table => render_table(result),
        RenderType::List => render_list(result),
        RenderType::Text => render_text(result),
        RenderType::Structured { format, .. } => {
            render_structured(result, format)
        }
    }
}

/// Render as JSON
fn render_json(result: &ToolResult) -> String {
    if let Some(data) = &result.data {
        serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    }
}

/// Render as Markdown
fn render_markdown(result: &ToolResult) -> String {
    if let Some(data) = &result.data {
        match data {
            Value::Array(items) => {
                items.iter()
                    .enumerate()
                    .map(|(i, item)| format!("## Item {}\n\n```json\n{}\n```\n", i + 1, serde_json::to_string_pretty(item).unwrap_or_default()))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Value::Object(obj) => {
                format!("## Result\n\n```json\n{}\n```\n", serde_json::to_string_pretty(data).unwrap_or_default())
            }
            _ => format!("```json\n{}\n```\n", serde_json::to_string_pretty(data).unwrap_or_default()),
        }
    } else {
        "No data available".to_string()
    }
}

/// Render as HTML
fn render_html(result: &ToolResult) -> String {
    if let Some(data) = &result.data {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Tool Result</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        pre {{ background: #f5f5f5; padding: 10px; border-radius: 5px; }}
    </style>
</head>
<body>
    <h1>Tool Result</h1>
    <pre>{}</pre>
</body>
</html>"#,
            serde_json::to_string_pretty(data).unwrap_or_default()
        )
    } else {
        "<p>No data available</p>".to_string()
    }
}

/// Render as table
fn render_table(result: &ToolResult) -> String {
    if let Some(Value::Array(items)) = &result.data {
        if items.is_empty() {
            return "No items to display".to_string();
        }
        
        // Extract column names from first item
        if let Some(Value::Object(first)) = items.first() {
            let columns: Vec<&String> = first.keys().collect();
            
            // Build markdown table
            let mut table = String::new();
            
            // Header
            table.push_str("| ");
            let column_strs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
            table.push_str(&column_strs.join(" | "));
            table.push_str(" |\n");
            
            // Separator
            table.push_str("| ");
            for _ in &columns {
                table.push_str("--- | ");
            }
            table.push_str("\n");
            
            // Rows
            for item in items {
                if let Value::Object(obj) = item {
                    table.push_str("| ");
                    let row: Vec<String> = columns.iter()
                        .map(|col| {
                            obj.get(*col)
                                .map(|v| format_value_cell(v))
                                .unwrap_or_else(|| "-".to_string())
                        })
                        .collect();
                    table.push_str(&row.join(" | "));
                    table.push_str(" |\n");
                }
            }
            
            table
        } else {
            render_json(result)
        }
    } else {
        render_json(result)
    }
}

/// Render as list
fn render_list(result: &ToolResult) -> String {
    if let Some(Value::Array(items)) = &result.data {
        items.iter()
            .enumerate()
            .map(|(i, item)| format!("{}. {}", i + 1, format_value_simple(item)))
            .collect::<Vec<_>>()
            .join("\n")
    } else if let Some(data) = &result.data {
        format!("• {}", format_value_simple(data))
    } else {
        "No items".to_string()
    }
}

/// Render as plain text
fn render_text(result: &ToolResult) -> String {
    if let Some(data) = &result.data {
        format_value_simple(data)
    } else {
        "No data available".to_string()
    }
}

/// Render with structured format
fn render_structured(result: &ToolResult, format: &str) -> String {
    match format {
        "csv" => render_csv(result),
        "tsv" => render_tsv(result),
        _ => render_json(result),
    }
}

/// Render as CSV
fn render_csv(result: &ToolResult) -> String {
    if let Some(Value::Array(items)) = &result.data {
        if items.is_empty() {
            return String::new();
        }
        
        if let Some(Value::Object(first)) = items.first() {
            let columns: Vec<&String> = first.keys().collect();
            
            let mut csv = String::new();
            
            // Header
            let column_strs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
            csv.push_str(&column_strs.join(","));
            csv.push('\n');
            
            // Rows
            for item in items {
                if let Value::Object(obj) = item {
                    let row: Vec<String> = columns.iter()
                        .map(|col| {
                            obj.get(*col)
                                .map(|v| escape_csv_value(v))
                                .unwrap_or_else(|| String::new())
                        })
                        .collect();
                    csv.push_str(&row.join(","));
                    csv.push('\n');
                }
            }
            
            csv
        } else {
            render_json(result)
        }
    } else {
        render_json(result)
    }
}

/// Render as TSV
fn render_tsv(result: &ToolResult) -> String {
    if let Some(Value::Array(items)) = &result.data {
        if items.is_empty() {
            return String::new();
        }
        
        if let Some(Value::Object(first)) = items.first() {
            let columns: Vec<&String> = first.keys().collect();
            
            let mut tsv = String::new();
            
            // Header
            let column_strs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
            tsv.push_str(&column_strs.join("\t"));
            tsv.push('\n');
            
            // Rows
            for item in items {
                if let Value::Object(obj) = item {
                    let row: Vec<String> = columns.iter()
                        .map(|col| {
                            obj.get(*col)
                                .map(|v| format_value_simple(v))
                                .unwrap_or_else(|| String::new())
                        })
                        .collect();
                    tsv.push_str(&row.join("\t"));
                    tsv.push('\n');
                }
            }
            
            tsv
        } else {
            render_json(result)
        }
    } else {
        render_json(result)
    }
}

// Helper functions

fn format_value_cell(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "-".to_string(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_default()
        }
    }
}

fn format_value_simple(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(arr) => {
            arr.iter()
                .map(format_value_simple)
                .collect::<Vec<_>>()
                .join(", ")
        }
        Value::Object(obj) => {
            obj.iter()
                .map(|(k, v)| format!("{}: {}", k, format_value_simple(v)))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

fn escape_csv_value(value: &Value) -> String {
    let s = format_value_simple(value);
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

