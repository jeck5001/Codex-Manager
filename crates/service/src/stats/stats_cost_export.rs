use codexmanager_core::rpc::types::{CostExportResult, CostSummaryParams, CostSummaryResult};

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn push_csv_row(lines: &mut Vec<String>, columns: &[String]) {
    lines.push(
        columns
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );
}

fn build_cost_export_csv(summary: &CostSummaryResult) -> String {
    let mut lines = Vec::new();
    lines.push("section,dimension,dimensionValue,requestCount,inputTokens,cachedInputTokens,outputTokens,totalTokens,estimatedCostUsd".to_string());

    push_csv_row(
        &mut lines,
        &[
            "total".to_string(),
            "range".to_string(),
            format!("{}..{}", summary.range_start, summary.range_end),
            summary.total.request_count.to_string(),
            summary.total.input_tokens.to_string(),
            summary.total.cached_input_tokens.to_string(),
            summary.total.output_tokens.to_string(),
            summary.total.total_tokens.to_string(),
            format!("{:.6}", summary.total.estimated_cost_usd),
        ],
    );

    for item in &summary.by_key {
        push_csv_row(
            &mut lines,
            &[
                "byKey".to_string(),
                "keyId".to_string(),
                item.key_id.clone(),
                item.request_count.to_string(),
                item.input_tokens.to_string(),
                item.cached_input_tokens.to_string(),
                item.output_tokens.to_string(),
                item.total_tokens.to_string(),
                format!("{:.6}", item.estimated_cost_usd),
            ],
        );
    }
    for item in &summary.by_model {
        push_csv_row(
            &mut lines,
            &[
                "byModel".to_string(),
                "model".to_string(),
                item.model.clone(),
                item.request_count.to_string(),
                item.input_tokens.to_string(),
                item.cached_input_tokens.to_string(),
                item.output_tokens.to_string(),
                item.total_tokens.to_string(),
                format!("{:.6}", item.estimated_cost_usd),
            ],
        );
    }
    for item in &summary.by_day {
        push_csv_row(
            &mut lines,
            &[
                "byDay".to_string(),
                "day".to_string(),
                item.day.clone(),
                item.request_count.to_string(),
                item.input_tokens.to_string(),
                item.cached_input_tokens.to_string(),
                item.output_tokens.to_string(),
                item.total_tokens.to_string(),
                format!("{:.6}", item.estimated_cost_usd),
            ],
        );
    }

    lines.join("\n") + "\n"
}

pub(crate) fn export_cost_summary(params: CostSummaryParams) -> Result<CostExportResult, String> {
    let summary = crate::stats_cost_summary::read_cost_summary(params)?;
    Ok(CostExportResult {
        file_name: format!("codexmanager-costs-{}.csv", summary.preset),
        content: build_cost_export_csv(&summary),
    })
}

#[cfg(test)]
mod tests {
    use super::build_cost_export_csv;
    use codexmanager_core::rpc::types::{
        CostSummaryDayItem, CostSummaryKeyItem, CostSummaryModelItem, CostSummaryResult,
        CostUsageSummaryResult,
    };

    #[test]
    fn cost_export_csv_contains_all_summary_sections() {
        let csv = build_cost_export_csv(&CostSummaryResult {
            preset: "custom".to_string(),
            range_start: 1,
            range_end: 2,
            total: CostUsageSummaryResult {
                request_count: 3,
                input_tokens: 10,
                cached_input_tokens: 2,
                output_tokens: 5,
                total_tokens: 13,
                estimated_cost_usd: 0.42,
            },
            by_key: vec![CostSummaryKeyItem {
                key_id: "key-a".to_string(),
                request_count: 2,
                input_tokens: 8,
                cached_input_tokens: 1,
                output_tokens: 4,
                total_tokens: 11,
                estimated_cost_usd: 0.3,
            }],
            by_model: vec![CostSummaryModelItem {
                model: "o3".to_string(),
                request_count: 2,
                input_tokens: 8,
                cached_input_tokens: 1,
                output_tokens: 4,
                total_tokens: 11,
                estimated_cost_usd: 0.3,
            }],
            by_day: vec![CostSummaryDayItem {
                day: "2026-03-22".to_string(),
                request_count: 3,
                input_tokens: 10,
                cached_input_tokens: 2,
                output_tokens: 5,
                total_tokens: 13,
                estimated_cost_usd: 0.42,
            }],
        });

        assert!(csv.contains("section,dimension,dimensionValue"));
        assert!(csv.contains("byKey,keyId,key-a"));
        assert!(csv.contains("byModel,model,o3"));
        assert!(csv.contains("byDay,day,2026-03-22"));
    }
}
