mod marketing_llm;
mod render;

use std::{env, fs};

use composure_calibration::CalibrationResult;
use composure_core::{
    build_deterministic_report, compare_monte_carlo_results, summarize_run, ComparisonConfig,
    CounterfactualResult, DeterministicReport, ExperimentBundle, ExperimentExecutionConfig,
    MonteCarloResult, RunSummary, SweepExecutionResult, TrajectoryComparison,
};
use composure_market::{MarketSimEngine, MarketSimulationConfig, MarketSimulationResult, Validate};
use composure_marketing::{
    simulate_marketing, simulate_marketing_v2, simulate_synthetic_market,
    CampaignVariantDefinition, ChannelAssumption, EvaluatorConfig, MarketingSimulationRequest,
    MarketingSimulationRequestV2, MarketingSimulationResultV2, MetricKind, ProductFrictionPrior,
    SegmentBlueprint, SegmentOverlapAssumption, SyntheticMarketMetadata, SyntheticMarketPackage,
    SyntheticMarketSimulationResult, SyntheticObservedOutcome, SyntheticScenarioDefinition,
    ValueDriverPrior,
};
use composure_runtime::{
    default_run_id, load_counterfactual, load_pack, load_pack_for_run,
    run_counterfactual_definition, run_pack, run_pack_counterfactual,
};
use marketing_llm::simulate_marketing_v2_assisted;
use render::{
    format_bundle, format_calibration, format_comparison, format_counterfactual_result,
    format_report, format_summary, format_sweep, render_bundle_markdown, render_calibration_csv,
    render_calibration_markdown, render_market_report_markdown,
    render_marketing_v2_compare_markdown, render_marketing_v2_report_markdown,
    render_report_markdown, render_sweep_csv, render_sweep_markdown, render_sweep_summary_markdown,
    render_synthetic_market_report_markdown,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

fn main() {
    if let Err(err) = real_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), CliError> {
    let args: Vec<String> = env::args().collect();
    let output = run(&args)?;
    println!("{output}");
    Ok(())
}

fn run(args: &[String]) -> Result<String, CliError> {
    match args {
        [_bin] => Err(CliError::Usage(usage())),
        [_bin, command] if command == "help" || command == "--help" || command == "-h" => {
            Ok(usage())
        }
        [_bin, command, path] if command == "inspect-pack" => {
            let pack = load_pack(path).map_err(CliError::Pack)?;
            Ok(pack.summary())
        }
        [_bin, command, path] if command == "inspect-counterfactual" => {
            let counterfactual = load_counterfactual(path).map_err(CliError::CounterfactualSpec)?;
            Ok(counterfactual.summary())
        }
        [_bin, command, path] if command == "inspect-counterfactual-result" => {
            let result = read_json::<CounterfactualResult>(path)?;
            Ok(format_counterfactual_result(&result))
        }
        [_bin, command, path] if command == "validate-pack" => {
            let pack = load_pack(path).map_err(CliError::Pack)?;
            Ok(format!(
                "Pack valid: {} ({})",
                pack.definition.name, pack.definition.id
            ))
        }
        [_bin, command, path] if command == "inspect-synthetic-market" => {
            let package = load_synthetic_market_package_dir(path)?;
            Ok(format_synthetic_market_package(&package))
        }
        [_bin, command, path] if command == "validate-synthetic-market" => {
            let package = load_synthetic_market_package_dir(path)?;
            package
                .validate()
                .map_err(CliError::SyntheticMarketValidation)?;
            Ok(format!(
                "Synthetic market valid: {} (segments={}, variants={}, scenarios={})",
                package.market.name,
                package.segments.len(),
                package.campaign_variants.len(),
                package.scenarios.len()
            ))
        }
        [_bin, command, path] if command == "inspect-pack-counterfactual" => {
            let pack = load_pack(path).map_err(CliError::Pack)?;
            let counterfactual = pack
                .counterfactual_definition
                .as_ref()
                .ok_or(CliError::MissingPackCounterfactual)?;
            Ok(counterfactual.summary())
        }
        [_bin, command, path] if command == "validate-counterfactual" => {
            let counterfactual = load_counterfactual(path).map_err(CliError::CounterfactualSpec)?;
            Ok(format!(
                "Counterfactual valid: {} ({})",
                counterfactual.definition.name, counterfactual.definition.id
            ))
        }
        [_bin, command, path, tail @ ..] if command == "run-pack" => {
            let pack = load_pack_for_run(path).map_err(CliError::Pack)?;
            let bundle = run_pack(
                &pack,
                default_run_id(&pack),
                &ExperimentExecutionConfig::default(),
            )
            .map_err(CliError::PackRun)?;
            let output = serde_json::to_string_pretty(&bundle).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "run-pack-counterfactual" => {
            let pack = load_pack_for_run(path).map_err(CliError::Pack)?;
            let result = run_pack_counterfactual(&pack).map_err(CliError::PackCounterfactualRun)?;
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "run-counterfactual" => {
            let counterfactual = load_counterfactual(path).map_err(CliError::CounterfactualSpec)?;
            let result = run_counterfactual_definition(&counterfactual)
                .map_err(CliError::CounterfactualRun)?;
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-bundle" => {
            let bundle = read_json::<ExperimentBundle>(path)?;
            Ok(format_bundle(&bundle))
        }
        [_bin, command, path, tail @ ..] if command == "export-bundle-markdown" => {
            let bundle = read_json::<ExperimentBundle>(path)?;
            let output = render_bundle_markdown(&bundle);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-sweep" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            Ok(format_sweep(&result))
        }
        [_bin, command, path, tail @ ..] if command == "export-sweep-summary-markdown" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            let output = render_sweep_summary_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-summary" => {
            let summary = read_json::<RunSummary>(path)?;
            Ok(format_summary(&summary))
        }
        [_bin, command, path] if command == "inspect-report" => {
            let report = read_json::<DeterministicReport>(path)?;
            Ok(format_report(&report))
        }
        [_bin, command, path, tail @ ..] if command == "export-report-markdown" => {
            let report = read_json::<DeterministicReport>(path)?;
            let output = render_report_markdown(&report);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "summarize-monte-carlo" => {
            let result = read_json::<MonteCarloResult>(path)?;
            let summary = summarize_run(Some(&result), None);
            let output = serde_json::to_string_pretty(&summary).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-calibration" => {
            let calibration = read_json::<CalibrationResult>(path)?;
            Ok(format_calibration(&calibration))
        }
        [_bin, command, path, tail @ ..]
            if command == "export-calibration-candidates"
                || command == "export-calibration-csv" =>
        {
            let calibration = read_json::<CalibrationResult>(path)?;
            let output = render_calibration_csv(&calibration);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "export-calibration-candidates-markdown" => {
            let calibration = read_json::<CalibrationResult>(path)?;
            let output = render_calibration_markdown(&calibration);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-compare" => {
            let comparison = read_json::<TrajectoryComparison>(path)?;
            Ok(format_comparison(&comparison))
        }
        [_bin, command, path, tail @ ..]
            if command == "export-sweep-samples" || command == "export-sweep-csv" =>
        {
            let result = read_json::<SweepExecutionResult>(path)?;
            let output = render_sweep_csv(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "export-sweep-samples-markdown" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            let output = render_sweep_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, bundle_path, run_id, tail @ ..] if command == "summarize-bundle-run" => {
            let bundle = read_json::<ExperimentBundle>(bundle_path)?;
            let run = bundle
                .runs
                .iter()
                .find(|run| run.run_id == *run_id)
                .ok_or_else(|| CliError::RunNotFound {
                    bundle_path: bundle_path.clone(),
                    run_id: run_id.clone(),
                })?;
            let outcome = run
                .outcome
                .as_ref()
                .ok_or_else(|| CliError::MissingRunOutcome {
                    bundle_path: bundle_path.clone(),
                    run_id: run_id.clone(),
                })?;
            let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());
            let output = serde_json::to_string_pretty(&summary).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, baseline_path, candidate_path, tail @ ..]
            if command == "compare-monte-carlo" =>
        {
            let baseline = read_json::<MonteCarloResult>(baseline_path)?;
            let candidate = read_json::<MonteCarloResult>(candidate_path)?;
            let options = parse_compare_options(tail)?;
            let comparison = compare_monte_carlo_results(&baseline, &candidate, &options.config)
                .map_err(CliError::Compare)?;
            let output =
                serde_json::to_string_pretty(&comparison).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, command, baseline_path, candidate_path, tail @ ..] if command == "build-report" => {
            let baseline = read_json::<RunSummary>(baseline_path)?;
            let candidate = read_json::<RunSummary>(candidate_path)?;
            let options = parse_build_report_options(tail)?;
            let comparison = match options.comparison_path.as_deref() {
                Some(path) => Some(read_json::<TrajectoryComparison>(path)?),
                None => None,
            };
            let report = build_deterministic_report(&baseline, &candidate, comparison.as_ref());
            let output = serde_json::to_string_pretty(&report).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, command, path, tail @ ..] if command == "simulate-marketing" => {
            let request = read_json::<MarketingSimulationRequest>(path)?;
            let result = simulate_marketing(&request).map_err(CliError::MarketingSimulation)?;
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "simulate-marketing-v2" => {
            let request = read_json::<MarketingSimulationRequestV2>(path)?;
            let result = simulate_marketing_v2(&request).map_err(CliError::MarketingSimulation)?;
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "simulate-marketing-v2-assisted" => {
            let mut request = read_json::<MarketingSimulationRequestV2>(path)?;
            let options = parse_marketing_v2_assisted_options(tail)?;
            apply_marketing_v2_assisted_overrides(&mut request, &options);
            let result =
                simulate_marketing_v2_assisted(&request).map_err(CliError::MarketingLlm)?;
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, command, path, scenario_id, tail @ ..] if command == "simulate-synthetic-market" => {
            let package = load_synthetic_market_package_dir(path)?;
            let result = simulate_synthetic_market(&package, scenario_id)
                .map_err(CliError::SyntheticMarketSimulation)?;
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, tail @ ..] if command == "compare-marketing-v2-assisted" => {
            let (request_paths, options) = parse_marketing_v2_compare_inputs(tail)?;
            let report = build_marketing_v2_compare_report(&request_paths, &options)?;
            let output = serde_json::to_string_pretty(&report).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, command, path, tail @ ..] if command == "export-marketing-v2-report-markdown" => {
            let result = read_json::<MarketingSimulationResultV2>(path)?;
            let output = render_marketing_v2_report_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..]
            if command == "export-synthetic-market-report-markdown" =>
        {
            let result = read_json::<SyntheticMarketSimulationResult>(path)?;
            let output = render_synthetic_market_report_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "export-marketing-v2-compare-markdown" => {
            let report = read_json::<MarketingV2ComparisonReport>(path)?;
            let output = render_marketing_v2_compare_markdown(&report);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "market-sim" => {
            let config = read_json::<MarketSimulationConfig>(path)?;
            let errors = config.validate();
            if !errors.is_empty() {
                let messages: Vec<String> = errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect();
                return Err(CliError::MarketSimulation(messages.join("; ")));
            }
            let options = parse_market_sim_options(tail)?;
            let mut engine = MarketSimEngine::new(config);
            let result = engine.run();
            let output = serde_json::to_string_pretty(&result).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, command, path, tail @ ..] if command == "export-market-report" => {
            let result = read_json::<MarketSimulationResult>(path)?;
            let output = render_market_report_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, ..] => Err(CliError::UnknownCommand { usage: usage() }),
        [] => Err(CliError::Usage(usage())),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  composure inspect-pack <path>",
        "  composure inspect-counterfactual <path>",
        "  composure inspect-counterfactual-result <path>",
        "  composure inspect-pack-counterfactual <path>",
        "  composure validate-pack <path>",
        "  composure inspect-synthetic-market <dir>",
        "  composure validate-synthetic-market <dir>",
        "  composure validate-counterfactual <path>",
        "  composure run-pack <path> [--output <path>]",
        "  composure run-pack-counterfactual <path> [--output <path>]",
        "  composure run-counterfactual <path> [--output <path>]",
        "  composure inspect-bundle <path>",
        "  composure export-bundle-markdown <path> [--output <path>]",
        "  composure inspect-sweep <path>",
        "  composure export-sweep-summary-markdown <path> [--output <path>]",
        "  composure inspect-summary <path>",
        "  composure inspect-report <path>",
        "  composure export-report-markdown <path> [--output <path>]",
        "  composure summarize-monte-carlo <path> [--output <path>]",
        "  composure summarize-bundle-run <bundle-path> <run-id> [--output <path>]",
        "  composure inspect-calibration <path>",
        "  composure export-calibration-candidates <path> [--output <path>]",
        "  composure export-calibration-candidates-markdown <path> [--output <path>]",
        "  composure inspect-compare <path>",
        "  composure export-sweep-samples <path> [--output <path>]",
        "  composure export-sweep-samples-markdown <path> [--output <path>]",
        "  composure compare-monte-carlo <baseline-path> <candidate-path> [flags] [--output <path>]",
        "  composure build-report <baseline-summary-path> <candidate-summary-path> [--comparison <path>] [--output <path>]",
        "  composure simulate-marketing <request-path> [--output <path>]",
        "  composure simulate-marketing-v2 <request-path> [--output <path>]",
        "  composure simulate-marketing-v2-assisted <request-path> [--provider <name>] [--model <name>] [--reasoning-effort <level>] [--output <path>]",
        "  composure simulate-synthetic-market <dir> <scenario-id> [--output <path>]",
        "  composure compare-marketing-v2-assisted <request-path> <request-path> [more-paths...] [--provider <name>] [--model <name>] [--reasoning-effort <level>] [--output <path>]",
        "  composure export-marketing-v2-report-markdown <path> [--output <path>]",
        "  composure export-synthetic-market-report-markdown <path> [--output <path>]",
        "  composure export-marketing-v2-compare-markdown <path> [--output <path>]",
        "  composure market-sim <config-path> [--output <path>]",
        "  composure export-market-report <result-path> [--output <path>]",
        "",
        "Commands:",
        "  inspect-pack   Read a pack directory or manifest and print a compiled summary",
        "  inspect-counterfactual   Read a CounterfactualDefinition JSON artifact and print a summary",
        "  inspect-counterfactual-result   Read a CounterfactualResult JSON artifact and print a summary",
        "  inspect-pack-counterfactual   Resolve and print the pack-managed counterfactual summary",
        "  validate-pack  Validate a pack directory or manifest and its referenced artifacts",
        "  inspect-synthetic-market  Read a synthetic market directory and print a summary",
        "  validate-synthetic-market  Validate a synthetic market directory and its referenced scenario/config files",
        "  validate-counterfactual  Validate a CounterfactualDefinition JSON artifact",
        "  run-pack  Execute a pack with its built-in runtime model and emit an ExperimentBundle artifact",
        "  run-pack-counterfactual  Execute a pack-managed counterfactual definition and emit a CounterfactualResult artifact",
        "  run-counterfactual  Execute a CounterfactualDefinition JSON artifact and emit a CounterfactualResult artifact",
        "  inspect-bundle   Read an ExperimentBundle JSON artifact and print a summary",
        "  export-bundle-markdown  Convert an ExperimentBundle JSON artifact into markdown",
        "  inspect-sweep    Read a SweepExecutionResult JSON artifact and print a summary",
        "  export-sweep-summary-markdown  Convert a SweepExecutionResult JSON artifact into markdown summary",
        "  inspect-summary  Read a RunSummary JSON artifact and print a summary",
        "  inspect-report   Read a DeterministicReport JSON artifact and print a summary",
        "  export-report-markdown  Convert a DeterministicReport JSON artifact into markdown",
        "  summarize-monte-carlo  Convert a MonteCarloResult JSON artifact into a RunSummary JSON artifact",
        "  summarize-bundle-run   Extract and summarize one run from an ExperimentBundle JSON artifact",
        "  inspect-calibration  Read a CalibrationResult JSON artifact and print a summary",
        "  export-calibration-candidates  Convert a CalibrationResult JSON artifact into CSV",
        "  export-calibration-candidates-markdown  Convert a CalibrationResult JSON artifact into markdown",
        "  inspect-compare  Read a TrajectoryComparison JSON artifact and print a summary",
        "  export-sweep-samples  Convert a SweepExecutionResult JSON artifact into CSV",
        "  export-sweep-samples-markdown  Convert a SweepExecutionResult JSON artifact into markdown",
        "  compare-monte-carlo  Compare two MonteCarloResult JSON artifacts and emit JSON",
        "  build-report   Build a DeterministicReport JSON artifact from two RunSummary artifacts",
        "  simulate-marketing   Execute the marketing adapter against a request JSON payload",
        "  simulate-marketing-v2   Execute the marketing V2 adapter against a request JSON payload",
        "  simulate-marketing-v2-assisted   Execute the marketing V2 adapter and enrich the result with an LLM analysis",
        "  simulate-synthetic-market   Execute the synthetic market cohort simulator for one scenario",
        "  compare-marketing-v2-assisted   Execute multiple assisted marketing V2 scenarios and rank them side by side",
        "  export-marketing-v2-report-markdown   Convert a MarketingSimulationResultV2 JSON artifact into markdown",
        "  export-synthetic-market-report-markdown   Convert a SyntheticMarketSimulationResult JSON artifact into markdown",
        "  export-marketing-v2-compare-markdown   Convert a MarketingV2ComparisonReport JSON artifact into markdown",
        "  market-sim   Execute the buyer-level market simulation kernel",
        "  export-market-report   Convert a MarketSimulationResult JSON artifact into markdown",
        "Compare/build flags:",
        "  --divergence-threshold <float>",
        "  --sustained-steps <usize>",
        "  --equality-epsilon <float>",
        "  --failure-threshold <float>",
        "  --comparison <path>",
        "  --output <path>",
        "Assisted marketing flags:",
        "  --provider <name>",
        "  --model <name>",
        "  --reasoning-effort <level>",
    ]
    .join("\n")
}

fn read_json<T>(path: &str) -> Result<T, CliError>
where
    T: serde::de::DeserializeOwned,
{
    let raw = fs::read_to_string(path).map_err(|source| CliError::ReadFile {
        path: path.into(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CliError::ParseJson {
        path: path.into(),
        source,
    })
}

#[derive(Debug, Deserialize)]
struct SyntheticSegmentsFile {
    market: SyntheticMarketMetadata,
    #[serde(default)]
    segments: Vec<SegmentBlueprint>,
    #[serde(default)]
    overlap_assumptions: Vec<SegmentOverlapAssumption>,
}

#[derive(Debug, Deserialize)]
struct SyntheticFrictionFile {
    #[serde(default)]
    frictions: Vec<ProductFrictionPrior>,
    #[serde(default)]
    value_drivers: Vec<ValueDriverPrior>,
}

#[derive(Debug, Deserialize)]
struct SyntheticChannelsFile {
    #[serde(default)]
    channels: Vec<ChannelAssumption>,
}

#[derive(Debug, Deserialize)]
struct SyntheticVariantsFile {
    #[serde(default)]
    variants: Vec<CampaignVariantDefinition>,
}

#[derive(Debug, Deserialize)]
struct SyntheticObservedOutcomesFile {
    #[serde(default)]
    outcomes: Vec<SyntheticObservedOutcome>,
}

fn load_synthetic_market_package_dir(path: &str) -> Result<SyntheticMarketPackage, CliError> {
    let segments_path = format!("{path}/config/mirrorlife-segment-blueprints.json");
    let friction_path = format!("{path}/config/mirrorlife-product-friction-priors.json");
    let channels_path = format!("{path}/config/mirrorlife-channel-assumptions.json");
    let variants_path = format!("{path}/config/mirrorlife-campaign-variants.json");
    let observed_outcomes_path =
        format!("{path}/observed-outcomes/mirrorlife-observed-outcomes.template.json");
    let scenarios_dir = format!("{path}/scenarios");

    let segments = read_json::<SyntheticSegmentsFile>(&segments_path)?;
    let friction = read_json::<SyntheticFrictionFile>(&friction_path)?;
    let channels = read_json::<SyntheticChannelsFile>(&channels_path)?;
    let variants = read_json::<SyntheticVariantsFile>(&variants_path)?;
    let observed_outcomes = if std::path::Path::new(&observed_outcomes_path).exists() {
        read_json::<SyntheticObservedOutcomesFile>(&observed_outcomes_path)?.outcomes
    } else {
        Vec::new()
    };

    let mut scenario_paths = fs::read_dir(&scenarios_dir)
        .map_err(|source| CliError::ReadFile {
            path: scenarios_dir.clone(),
            source,
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    scenario_paths.sort();

    let mut scenarios = Vec::new();
    for scenario_path in scenario_paths {
        let scenario_path_string = scenario_path.to_string_lossy().to_string();
        scenarios.push(read_json::<SyntheticScenarioDefinition>(
            &scenario_path_string,
        )?);
    }

    Ok(SyntheticMarketPackage {
        market: segments.market,
        segments: segments.segments,
        overlap_assumptions: segments.overlap_assumptions,
        frictions: friction.frictions,
        value_drivers: friction.value_drivers,
        channels: channels.channels,
        campaign_variants: variants.variants,
        scenarios,
        observed_outcomes,
    })
}

fn format_synthetic_market_package(package: &SyntheticMarketPackage) -> String {
    let scenario_ids = package
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Synthetic Market: {} ({})\nSegments: {}\nVariants: {}\nScenarios: {}\nScenario IDs: {}",
        package.market.name,
        package.market.version.as_deref().unwrap_or("unknown"),
        package.segments.len(),
        package.campaign_variants.len(),
        package.scenarios.len(),
        scenario_ids
    )
}

#[derive(Debug)]
struct CompareOptions {
    config: ComparisonConfig,
    output_path: Option<String>,
}

#[derive(Debug)]
struct BuildReportOptions {
    comparison_path: Option<String>,
    output_path: Option<String>,
}

#[derive(Debug, Default)]
struct MarketingV2AssistedOptions {
    provider: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    output_path: Option<String>,
}

#[derive(Debug, Default)]
struct MarketSimOptions {
    output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketingV2ComparisonReport {
    comparison_id: String,
    compared_requests: Vec<String>,
    provider: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    portfolio_recommendation: Vec<String>,
    repeated_winner_patterns: Vec<String>,
    scenarios: Vec<MarketingV2ComparisonScenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketingV2ComparisonScenario {
    request_path: String,
    scenario_name: String,
    scenario_type: String,
    simulation_id: String,
    overall_score: u32,
    winner_approach_id: String,
    winner_overall_score: u32,
    runner_up_approach_id: Option<String>,
    runner_up_overall_score: Option<u32>,
    score_gap_vs_runner_up: Option<i32>,
    strongest_metric_label: Option<String>,
    strongest_metric_score: Option<u32>,
    #[serde(default)]
    metric_deltas: Vec<MarketingV2MetricDelta>,
    #[serde(default)]
    strongest_positive_delta_metric: Option<String>,
    #[serde(default)]
    strongest_positive_delta_value: Option<i32>,
    #[serde(default)]
    weakest_delta_metric: Option<String>,
    #[serde(default)]
    weakest_delta_value: Option<i32>,
    recommended_next_experiments: Vec<String>,
    llm_executive_summary: Vec<String>,
    llm_consensus_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketingV2MetricDelta {
    #[serde(default)]
    metric: Option<MetricKind>,
    label: String,
    score: u32,
    delta_vs_compare_average: i32,
    #[serde(default)]
    delta_vs_compare_leader: i32,
    #[serde(default)]
    compare_set_rank: usize,
    #[serde(default)]
    compare_set_size: usize,
    #[serde(default)]
    leading_scenarios: Vec<String>,
}

fn parse_compare_options(args: &[String]) -> Result<CompareOptions, CliError> {
    let mut config = ComparisonConfig::default();
    let mut output_path = None;
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| CliError::MissingFlagValue(flag.clone()))?;

        match flag.as_str() {
            "--divergence-threshold" => {
                config.divergence_threshold = parse_flag(value, flag)?;
            }
            "--sustained-steps" => {
                config.sustained_steps = parse_flag(value, flag)?;
            }
            "--equality-epsilon" => {
                config.equality_epsilon = parse_flag(value, flag)?;
            }
            "--failure-threshold" => {
                config.failure_threshold = Some(parse_flag(value, flag)?);
            }
            "--output" => {
                output_path = Some(value.clone());
            }
            _ => return Err(CliError::UnknownFlag(flag.clone())),
        }

        index += 2;
    }

    config.validate().map_err(CliError::Compare)?;
    Ok(CompareOptions {
        config,
        output_path,
    })
}

fn parse_build_report_options(args: &[String]) -> Result<BuildReportOptions, CliError> {
    let mut comparison_path = None;
    let mut output_path = None;
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| CliError::MissingFlagValue(flag.clone()))?;

        match flag.as_str() {
            "--comparison" => {
                comparison_path = Some(value.clone());
            }
            "--output" => {
                output_path = Some(value.clone());
            }
            _ => return Err(CliError::UnknownFlag(flag.clone())),
        }

        index += 2;
    }

    Ok(BuildReportOptions {
        comparison_path,
        output_path,
    })
}

fn parse_marketing_v2_assisted_options(
    args: &[String],
) -> Result<MarketingV2AssistedOptions, CliError> {
    let mut options = MarketingV2AssistedOptions::default();
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| CliError::MissingFlagValue(flag.clone()))?;

        match flag.as_str() {
            "--provider" => {
                options.provider = Some(value.clone());
            }
            "--model" => {
                options.model = Some(value.clone());
            }
            "--reasoning-effort" => {
                options.reasoning_effort = Some(value.clone());
            }
            "--output" => {
                options.output_path = Some(value.clone());
            }
            _ => return Err(CliError::UnknownFlag(flag.clone())),
        }

        index += 2;
    }

    Ok(options)
}

fn parse_market_sim_options(args: &[String]) -> Result<MarketSimOptions, CliError> {
    let mut options = MarketSimOptions::default();
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| CliError::MissingFlagValue(flag.clone()))?;

        match flag.as_str() {
            "--output" => {
                options.output_path = Some(value.clone());
            }
            _ => return Err(CliError::UnknownFlag(flag.clone())),
        }

        index += 2;
    }

    Ok(options)
}

fn parse_marketing_v2_compare_inputs(
    args: &[String],
) -> Result<(Vec<String>, MarketingV2AssistedOptions), CliError> {
    let mut request_paths = Vec::new();
    let mut option_args = Vec::new();
    let mut in_flags = false;

    for arg in args {
        if arg.starts_with("--") {
            in_flags = true;
        }
        if in_flags {
            option_args.push(arg.clone());
        } else {
            request_paths.push(arg.clone());
        }
    }

    if request_paths.len() < 2 {
        return Err(CliError::Usage(
            "compare-marketing-v2-assisted requires at least two request paths".into(),
        ));
    }

    let options = parse_marketing_v2_assisted_options(&option_args)?;
    Ok((request_paths, options))
}

fn parse_output_flag(args: &[String]) -> Result<Option<&str>, CliError> {
    match args {
        [] => Ok(None),
        [flag, path] if flag == "--output" => Ok(Some(path.as_str())),
        [flag] => Err(CliError::MissingFlagValue(flag.clone())),
        [flag, ..] => Err(CliError::UnknownFlag(flag.clone())),
    }
}

fn parse_flag<T>(value: &str, flag: &str) -> Result<T, CliError>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| CliError::InvalidFlagValue {
        flag: flag.into(),
        value: value.into(),
    })
}

fn apply_marketing_v2_assisted_overrides(
    request: &mut MarketingSimulationRequestV2,
    options: &MarketingV2AssistedOptions,
) {
    if options.provider.is_none() && options.model.is_none() && options.reasoning_effort.is_none() {
        return;
    }

    let evaluator = request.evaluator.get_or_insert(EvaluatorConfig {
        provider: None,
        model: None,
        reasoning_effort: None,
    });
    if let Some(provider) = &options.provider {
        evaluator.provider = Some(provider.clone());
    }
    if let Some(model) = &options.model {
        evaluator.model = Some(model.clone());
    }
    if let Some(reasoning_effort) = &options.reasoning_effort {
        evaluator.reasoning_effort = Some(reasoning_effort.clone());
    }
}

fn build_marketing_v2_compare_report(
    request_paths: &[String],
    options: &MarketingV2AssistedOptions,
) -> Result<MarketingV2ComparisonReport, CliError> {
    let mut scenarios = Vec::with_capacity(request_paths.len());
    let mut common_llm_provider: Option<Option<String>> = None;
    let mut common_llm_model: Option<Option<String>> = None;
    let mut common_reasoning_effort: Option<Option<String>> = None;

    for request_path in request_paths {
        let mut request = read_json::<MarketingSimulationRequestV2>(request_path)?;
        apply_marketing_v2_assisted_overrides(&mut request, options);
        let result = simulate_marketing_v2_assisted(&request).map_err(CliError::MarketingLlm)?;
        let analysis_provider = result
            .llm_analysis
            .as_ref()
            .and_then(|analysis| analysis.provider.clone());
        let analysis_model = result
            .llm_analysis
            .as_ref()
            .map(|analysis| analysis.model.clone());
        let analysis_reasoning_effort = result
            .llm_analysis
            .as_ref()
            .and_then(|analysis| analysis.reasoning_effort.clone());
        merge_common_option(&mut common_llm_provider, analysis_provider);
        merge_common_option(&mut common_llm_model, analysis_model);
        merge_common_option(&mut common_reasoning_effort, analysis_reasoning_effort);
        let mut ranked = result.approach_results.iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .primary_scorecard
                .overall_score
                .cmp(&left.primary_scorecard.overall_score)
        });
        let winner = ranked
            .first()
            .copied()
            .ok_or_else(|| CliError::Usage("marketing compare result had no approaches".into()))?;
        let runner_up = ranked.get(1).copied();
        let top_metric = result
            .primary_scorecard
            .metrics
            .iter()
            .max_by_key(|metric| metric.score);

        scenarios.push(MarketingV2ComparisonScenario {
            request_path: request_path.clone(),
            scenario_name: result.scenario.name.clone(),
            scenario_type: serialize_scenario_type(&result.scenario.scenario_type),
            simulation_id: result.simulation_id.clone(),
            overall_score: result.primary_scorecard.overall_score,
            winner_approach_id: winner.approach_id.clone(),
            winner_overall_score: winner.primary_scorecard.overall_score,
            runner_up_approach_id: runner_up.map(|approach| approach.approach_id.clone()),
            runner_up_overall_score: runner_up
                .map(|approach| approach.primary_scorecard.overall_score),
            score_gap_vs_runner_up: runner_up.map(|approach| {
                winner.primary_scorecard.overall_score as i32
                    - approach.primary_scorecard.overall_score as i32
            }),
            strongest_metric_label: top_metric.map(|metric| metric.label.clone()),
            strongest_metric_score: top_metric.map(|metric| metric.score),
            metric_deltas: result
                .primary_scorecard
                .metrics
                .iter()
                .map(|metric| MarketingV2MetricDelta {
                    metric: Some(metric.metric.clone()),
                    label: metric.label.clone(),
                    score: metric.score,
                    delta_vs_compare_average: 0,
                    delta_vs_compare_leader: 0,
                    compare_set_rank: 0,
                    compare_set_size: 0,
                    leading_scenarios: Vec::new(),
                })
                .collect(),
            strongest_positive_delta_metric: None,
            strongest_positive_delta_value: None,
            weakest_delta_metric: None,
            weakest_delta_value: None,
            recommended_next_experiments: result
                .recommended_next_experiments
                .iter()
                .take(5)
                .cloned()
                .collect(),
            llm_executive_summary: result
                .llm_analysis
                .as_ref()
                .map(|analysis| analysis.executive_summary.iter().take(3).cloned().collect())
                .unwrap_or_default(),
            llm_consensus_summary: result
                .llm_analysis
                .as_ref()
                .map(|analysis| analysis.consensus_summary.iter().take(3).cloned().collect())
                .unwrap_or_default(),
        });
    }

    let mut metric_values = std::collections::BTreeMap::<String, Vec<(String, u32)>>::new();
    for scenario in &scenarios {
        for metric in &scenario.metric_deltas {
            metric_values
                .entry(metric.label.clone())
                .or_default()
                .push((scenario.scenario_name.clone(), metric.score));
        }
    }

    for scenario in &mut scenarios {
        for metric in &mut scenario.metric_deltas {
            if let Some(values) = metric_values.get(&metric.label) {
                let avg = values.iter().map(|(_, score)| *score as f64).sum::<f64>()
                    / values.len() as f64;
                let leader_score = values
                    .iter()
                    .map(|(_, score)| *score)
                    .max()
                    .unwrap_or(metric.score);
                metric.delta_vs_compare_average = metric.score as i32 - avg.round() as i32;
                metric.delta_vs_compare_leader = metric.score as i32 - leader_score as i32;
                metric.compare_set_rank = values
                    .iter()
                    .filter(|(_, score)| *score > metric.score)
                    .count()
                    + 1;
                metric.compare_set_size = values.len();
                metric.leading_scenarios = values
                    .iter()
                    .filter(|(_, score)| *score == leader_score)
                    .map(|(scenario_name, _)| scenario_name.clone())
                    .collect();
            }
        }
        scenario.metric_deltas.sort_by(|left, right| {
            right
                .delta_vs_compare_average
                .cmp(&left.delta_vs_compare_average)
                .then_with(|| {
                    right
                        .delta_vs_compare_leader
                        .cmp(&left.delta_vs_compare_leader)
                })
                .then_with(|| left.compare_set_rank.cmp(&right.compare_set_rank))
                .then_with(|| left.label.cmp(&right.label))
        });
        if let Some(best) = scenario
            .metric_deltas
            .iter()
            .find(|metric| metric.compare_set_size >= 2 && metric.delta_vs_compare_average > 0)
        {
            scenario.strongest_positive_delta_metric = Some(best.label.clone());
            scenario.strongest_positive_delta_value = Some(best.delta_vs_compare_average);
        }
        if let Some(worst) = scenario.metric_deltas.iter().min_by(|left, right| {
            left.delta_vs_compare_average
                .cmp(&right.delta_vs_compare_average)
                .then_with(|| {
                    left.delta_vs_compare_leader
                        .cmp(&right.delta_vs_compare_leader)
                })
                .then_with(|| left.label.cmp(&right.label))
        }) {
            if worst.compare_set_size >= 2 && worst.delta_vs_compare_average < 0 {
                scenario.weakest_delta_metric = Some(worst.label.clone());
                scenario.weakest_delta_value = Some(worst.delta_vs_compare_average);
            }
        }
    }

    scenarios.sort_by(|left, right| {
        right
            .overall_score
            .cmp(&left.overall_score)
            .then_with(|| left.scenario_name.cmp(&right.scenario_name))
    });

    let portfolio_recommendation = build_marketing_portfolio_recommendation(&scenarios);
    let repeated_winner_patterns = build_repeated_winner_patterns(&scenarios);

    Ok(MarketingV2ComparisonReport {
        comparison_id: format!("marketing-compare-{}", scenarios.len()),
        compared_requests: request_paths.to_vec(),
        provider: common_llm_provider.unwrap_or_else(|| options.provider.clone()),
        model: common_llm_model.unwrap_or_else(|| options.model.clone()),
        reasoning_effort: common_reasoning_effort
            .unwrap_or_else(|| options.reasoning_effort.clone()),
        portfolio_recommendation,
        repeated_winner_patterns,
        scenarios,
    })
}

fn merge_common_option<T>(slot: &mut Option<Option<T>>, candidate: Option<T>)
where
    T: PartialEq,
{
    match slot {
        None => *slot = Some(candidate),
        Some(current) if *current == candidate => {}
        Some(_) => *slot = Some(None),
    }
}

fn serialize_scenario_type(scenario_type: &composure_marketing::ScenarioType) -> String {
    serde_json::to_value(scenario_type)
        .ok()
        .and_then(|value| value.as_str().map(|value| value.to_string()))
        .unwrap_or_else(|| "custom".into())
}

fn build_marketing_portfolio_recommendation(
    scenarios: &[MarketingV2ComparisonScenario],
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(control) = scenarios.first() {
        lines.push(format!(
            "Use `{}` as the control scenario because it has the highest overall score at `{}`.",
            control.scenario_name, control.overall_score
        ));
    }
    if let Some(challenger) = scenarios.get(1) {
        lines.push(format!(
            "Use `{}` as the challenger scenario because it is the next-strongest option and creates a meaningful comparison against the control.",
            challenger.scenario_name
        ));
    }
    if let Some(best_gap) = scenarios
        .iter()
        .filter_map(|scenario| scenario.score_gap_vs_runner_up.map(|gap| (scenario, gap)))
        .max_by_key(|(_, gap)| *gap)
    {
        lines.push(format!(
            "`{}` had the clearest internal win with a `{}` point gap over its runner-up, so its positioning is currently the most decisive.",
            best_gap.0.scenario_name, best_gap.1
        ));
    }
    if let Some(control) = scenarios.first() {
        if let (Some(metric), Some(delta)) = (
            &control.strongest_positive_delta_metric,
            control.strongest_positive_delta_value,
        ) {
            lines.push(format!(
                "The control scenario's clearest cross-scenario edge was `{metric}`, where it ran `{delta}` points above the compare-set average."
            ));
        }
        if let (Some(metric), Some(delta)) =
            (&control.weakest_delta_metric, control.weakest_delta_value)
        {
            lines.push(format!(
                "The control scenario's main relative weakness was `{metric}`, where it sat `{delta}` points versus the compare-set average."
            ));
        }
    }
    lines
}

fn build_repeated_winner_patterns(scenarios: &[MarketingV2ComparisonScenario]) -> Vec<String> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for scenario in scenarios {
        *counts
            .entry(scenario.winner_approach_id.clone())
            .or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(winner, count)| {
            format!(
                "`{winner}` won in {count} scenarios, which suggests a repeated pattern worth keeping in the control set."
            )
        })
        .collect()
}

fn write_output(output: String, output_path: Option<&str>) -> Result<String, CliError> {
    match output_path {
        Some(path) => {
            fs::write(path, &output).map_err(|source| CliError::WriteFile {
                path: path.into(),
                source,
            })?;
            Ok(format!("Wrote artifact to {path}"))
        }
        None => Ok(output),
    }
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Usage(String),
    #[error("unknown command\n\n{usage}")]
    UnknownCommand { usage: String },
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse JSON from {path}: {source}")]
    ParseJson {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("run {run_id} was not found in bundle {bundle_path}")]
    RunNotFound { bundle_path: String, run_id: String },
    #[error("run {run_id} in bundle {bundle_path} does not have an outcome")]
    MissingRunOutcome { bundle_path: String, run_id: String },
    #[error("pack does not define a counterfactual_definition")]
    MissingPackCounterfactual,
    #[error("missing value for flag {0}")]
    MissingFlagValue(String),
    #[error("unknown flag {0}")]
    UnknownFlag(String),
    #[error("invalid value {value} for flag {flag}")]
    InvalidFlagValue { flag: String, value: String },
    #[error("comparison failed: {0}")]
    Compare(composure_core::CompareError),
    #[error("pack error: {0}")]
    Pack(composure_runtime::PackError),
    #[error("pack execution error: {0}")]
    PackRun(composure_runtime::PackRunError),
    #[error("pack counterfactual execution error: {0}")]
    PackCounterfactualRun(composure_runtime::PackCounterfactualRunError),
    #[error("counterfactual spec error: {0}")]
    CounterfactualSpec(composure_runtime::CounterfactualSpecError),
    #[error("counterfactual execution error: {0}")]
    CounterfactualRun(composure_runtime::CounterfactualRunError),
    #[error("marketing simulation error: {0}")]
    MarketingSimulation(composure_marketing::MarketingSimulationError),
    #[error("market simulation error: {0}")]
    MarketSimulation(String),
    #[error("synthetic market validation error: {0}")]
    SyntheticMarketValidation(composure_marketing::SyntheticMarketValidationError),
    #[error("synthetic market simulation error: {0}")]
    SyntheticMarketSimulation(composure_marketing::SyntheticMarketSimulationError),
    #[error("marketing LLM error: {0}")]
    MarketingLlm(marketing_llm::MarketingLlmError),
    #[error("failed to serialize JSON output: {0}")]
    SerializeJson(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        time::{SystemTime, UNIX_EPOCH},
    };

    use composure_calibration::{
        CalibrationCandidate, CalibrationCaseFailure, CalibrationConfig, CalibrationFailureMode,
        CalibrationObjective, CalibrationResult, ObservedTrajectory,
    };
    use composure_core::monte_carlo::PercentileBands;
    use composure_core::{
        Action, ActionType, ComposureSummary, ConditionalActionRule, ConditionalTrigger,
        ExperimentExecutionConfig, ExperimentOutcome, ExperimentParameterSet, ExperimentSpec,
        MonteCarloConfig, MonteCarloSummary, ParameterValue, Scenario, SensitivityConfig,
        SensitivityDirection, SensitivityKind, SweepCase, SweepDefinition, SweepExecutionResult,
        SweepFailureMode, SweepParameter, SweepRunnerConfig, SweepStrategy,
    };

    use super::*;

    fn sample_bundle() -> ExperimentBundle {
        let mut spec = ExperimentSpec::new(
            "exp-1",
            "Baseline",
            Scenario::new(
                "baseline",
                "Baseline",
                composure_core::SimState::zeros(1),
                5,
            ),
        );
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(10, 5, 42));
        let mut bundle = ExperimentBundle::new(spec);
        let mut parameter_set = ExperimentParameterSet::new(
            "variant-a",
            "Variant A",
            Scenario::new(
                "variant-a",
                "Variant A",
                composure_core::SimState::zeros(1),
                5,
            ),
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(10, 5, 7));
        bundle.add_parameter_set(parameter_set).unwrap();
        bundle
            .record_run(
                composure_core::ExperimentRunRecord::running("run-1", Some("variant-a"), Some(7))
                    .mark_completed(composure_core::ExperimentOutcome {
                        monte_carlo: None,
                        composure: None,
                        replay: None,
                        metadata: None,
                    }),
            )
            .unwrap();
        bundle
    }

    fn sample_monte_carlo(seed: u64, values: [f64; 3]) -> MonteCarloResult {
        MonteCarloResult {
            paths: vec![],
            percentiles: PercentileBands {
                p10: vec![values[0] - 0.04, values[1] - 0.04, values[2] - 0.04],
                p25: vec![values[0] - 0.02, values[1] - 0.02, values[2] - 0.02],
                p50: vec![values[0], values[1], values[2]],
                p75: vec![values[0] + 0.02, values[1] + 0.02, values[2] + 0.02],
                p90: vec![values[0] + 0.04, values[1] + 0.04, values[2] + 0.04],
            },
            mean_trajectory: values.into(),
            config: MonteCarloConfig::with_seed(10, 3, seed),
        }
    }

    fn sample_baseline_summary() -> RunSummary {
        RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 3,
                num_paths: 10,
                start: Some(0.84),
                end: Some(0.64),
                min: Some(0.64),
                max: Some(0.84),
                mean: Some(0.74),
                auc: Some(1.48),
                p10_end: Some(0.6),
                p50_end: Some(0.64),
                p90_end: Some(0.68),
                final_band_width: Some(0.08),
            }),
            composure: Some(ComposureSummary {
                archetype: composure_core::Archetype::CliffFaller,
                slope: -0.1,
                variance: 0.01,
                peak: 0.84,
                trough: 0.64,
                recovery_half_life: Some(2),
                residual_damage: 0.2,
                break_point: Some(1),
            }),
        }
    }

    fn sample_candidate_summary() -> RunSummary {
        RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 3,
                num_paths: 10,
                start: Some(0.86),
                end: Some(0.69),
                min: Some(0.69),
                max: Some(0.86),
                mean: Some(0.7733333333333333),
                auc: Some(1.545),
                p10_end: Some(0.65),
                p50_end: Some(0.69),
                p90_end: Some(0.73),
                final_band_width: Some(0.08),
            }),
            composure: Some(ComposureSummary {
                archetype: composure_core::Archetype::Phoenix,
                slope: -0.085,
                variance: 0.004822222222222221,
                peak: 0.86,
                trough: 0.69,
                recovery_half_life: Some(1),
                residual_damage: 0.17,
                break_point: Some(2),
            }),
        }
    }

    fn sample_comparison() -> TrajectoryComparison {
        composure_core::compare_trajectories(
            &[0.84, 0.74, 0.64],
            &[0.86, 0.77, 0.69],
            &ComparisonConfig::default(),
        )
        .unwrap()
    }

    fn sample_report() -> DeterministicReport {
        let baseline = sample_baseline_summary();
        let candidate = sample_candidate_summary();
        let comparison = sample_comparison();
        build_deterministic_report(&baseline, &candidate, Some(&comparison))
    }

    fn sample_calibration_result() -> CalibrationResult {
        let comparison = sample_comparison();
        let report = build_deterministic_report(
            &sample_baseline_summary(),
            &sample_candidate_summary(),
            Some(&comparison),
        );
        let mut parameters = BTreeMap::new();
        parameters.insert("dose".into(), ParameterValue::Int(3));
        let case = SweepCase {
            case_id: "dose-sweep-1".into(),
            parameters: parameters.clone(),
        };

        let mut parameter_set = ExperimentParameterSet::new(
            "ps-dose-sweep-1",
            "Dose Sweep 1",
            Scenario::new(
                "scenario-dose-sweep-1",
                "Scenario Dose Sweep 1",
                composure_core::SimState::zeros(1),
                3,
            ),
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(10, 3, 7));

        let run = composure_core::ExperimentRunRecord::running(
            "calibration-run-1",
            Some("ps-dose-sweep-1"),
            Some(7),
        )
        .mark_completed(ExperimentOutcome {
            monte_carlo: Some(sample_monte_carlo(7, [0.86, 0.77, 0.69])),
            composure: None,
            replay: None,
            metadata: None,
        });

        let mut failure_parameters = BTreeMap::new();
        failure_parameters.insert("dose".into(), ParameterValue::Int(2));

        CalibrationResult {
            definition: SweepDefinition {
                id: "dose-sweep".into(),
                name: "Dose Sweep".into(),
                parameters: vec![SweepParameter {
                    name: "dose".into(),
                    values: vec![
                        ParameterValue::Int(1),
                        ParameterValue::Int(2),
                        ParameterValue::Int(3),
                    ],
                }],
                strategy: SweepStrategy::Grid,
                sample_count: None,
                seed: None,
                metadata: None,
            },
            observed: ObservedTrajectory::new("obs-1", "Observed Recovery", vec![0.84, 0.74, 0.64]),
            observed_summary: sample_baseline_summary(),
            bundle: None,
            candidates: vec![CalibrationCandidate {
                case,
                parameter_set,
                run,
                summary: sample_candidate_summary(),
                comparison: comparison.clone(),
                report,
                score: comparison.metrics.rmse,
            }],
            failures: vec![CalibrationCaseFailure {
                case: SweepCase {
                    case_id: "dose-sweep-2".into(),
                    parameters: failure_parameters,
                },
                parameter_set_id: Some("ps-dose-sweep-2".into()),
                error: "dose rejected".into(),
            }],
            best_case_id: Some("dose-sweep-1".into()),
            best_parameter_set_id: Some("ps-dose-sweep-1".into()),
            best_score: Some(comparison.metrics.rmse),
            config: CalibrationConfig {
                objective: CalibrationObjective::Rmse,
                failure_mode: CalibrationFailureMode::Continue,
                ..CalibrationConfig::default()
            },
        }
    }

    fn sample_synthetic_market_result() -> SyntheticMarketSimulationResult {
        SyntheticMarketSimulationResult {
            market_name: "Mirrorlife".into(),
            scenario_id: "immediate_wedge_testing".into(),
            scenario_goal: "Determine the best immediate acquisition wedge.".into(),
            scenario_decision: "Choose control and challenger.".into(),
            total_buyers_simulated: 2048,
            market_funnel: composure_marketing::AggregateFunnelMetrics {
                buyers: 2048,
                clicks: 1515,
                signups: 833,
                activations: 248,
                retained: 42,
                paid_conversions: 0,
                click_rate: 0.76,
                signup_rate: 0.44,
                activation_rate: 0.14,
                retention_rate: 0.02,
                paid_conversion_rate: 0.0,
            },
            observed_data_summary: composure_marketing::ObservedDataSummary {
                records: 1,
                usable_records: 0,
                placeholder_records: 1,
                total_usable_sample_size: None,
                organic_sources: vec!["organic_waitlist".into()],
                paid_sources: vec![],
                acquisition_motion: "organic_only".into(),
                data_status: "placeholder_only".into(),
            },
            calibration_summary: vec![composure_marketing::SyntheticCalibrationSummary {
                variant_id: "peptide_glp1_wedge".into(),
                observed_records: 1,
                usable_observed_records: 0,
                placeholder_records: 1,
                observed_sample_size: None,
                compared_metrics: vec![],
                click_gap: None,
                signup_gap: None,
                activation_gap: None,
                retention_gap: None,
                paid_conversion_gap: None,
                note:
                    "Only placeholder outcomes exist for this variant; calibration is not live yet."
                        .into(),
            }],
            business_readiness: composure_marketing::BusinessReadinessSummary {
                acquisition_motion: "organic_only".into(),
                observed_data_status: "placeholder_only".into(),
                organic_readiness_score: 31,
                paid_readiness_score: 9,
                subscription_readiness_score: 5,
                current_focus: "Stay focused on organic channels and onboarding proof.".into(),
                gating_factors: vec![
                    "Observed outcomes are placeholders only.".into(),
                    "Activation is still too soft for scale.".into(),
                ],
            },
            recommended_control: "peptide_glp1_wedge".into(),
            recommended_challenger: Some("stack_intelligence".into()),
            ranked_variants: vec![composure_marketing::VariantScenarioScore {
                variant_id: "peptide_glp1_wedge".into(),
                role: Some("control".into()),
                overall_score: 91,
                weighted_segment_share: 0.29,
                funnel: composure_marketing::AggregateFunnelMetrics {
                    buyers: 2048,
                    clicks: 1515,
                    signups: 833,
                    activations: 248,
                    retained: 42,
                    paid_conversions: 0,
                    click_rate: 0.76,
                    signup_rate: 0.44,
                    activation_rate: 0.14,
                    retention_rate: 0.02,
                    paid_conversion_rate: 0.0,
                },
                strongest_segments: vec!["glp1_outcomes".into()],
                weakest_segments: vec!["privacy_first_tracker".into()],
                risk_flags: vec!["logging_risk".into()],
                segment_scores: vec![],
            }],
            segment_summaries: vec![composure_marketing::SegmentScenarioSummary {
                segment_id: "glp1_outcomes".into(),
                segment_name: "GLP-1 Outcomes Seeker".into(),
                effective_weight: 0.15,
                buyers_simulated: 800,
                best_variant_funnel: composure_marketing::AggregateFunnelMetrics {
                    buyers: 800,
                    clicks: 620,
                    signups: 360,
                    activations: 115,
                    retained: 21,
                    paid_conversions: 0,
                    click_rate: 0.77,
                    signup_rate: 0.45,
                    activation_rate: 0.14,
                    retention_rate: 0.03,
                    paid_conversion_rate: 0.0,
                },
                best_variant_id: "peptide_glp1_wedge".into(),
                best_score: 94,
                runner_up_variant_id: Some("stack_intelligence".into()),
                runner_up_score: Some(70),
            }],
            sampled_buyers: vec![composure_marketing::SyntheticBuyerSample {
                buyer_id: "buyer-1".into(),
                segment_id: "glp1_outcomes".into(),
                strongest_variant_id: "peptide_glp1_wedge".into(),
                strongest_variant_score: 94,
                runner_up_variant_id: Some("stack_intelligence".into()),
                runner_up_score: Some(70),
                proof_hunger: 82,
                manual_logging_tolerance: 61,
                privacy_sensitivity: 55,
                wearable_ownership: 34,
                subscription_willingness: 49,
                click_probability: 76,
                signup_probability: 44,
                activation_probability: 14,
                retention_probability: 2,
                paid_conversion_probability: 0,
                clicked: true,
                signed_up: true,
                activated: true,
                retained: false,
                converted_paid: false,
            }],
            notes: vec![
                "Control is strong on receptivity but weak on subscription readiness.".into(),
            ],
        }
    }

    fn sample_sweep_result() -> SweepExecutionResult {
        let mut parameters = BTreeMap::new();
        parameters.insert("dose".into(), ParameterValue::Int(2));

        SweepExecutionResult {
            definition: SweepDefinition {
                id: "sweep-1".into(),
                name: "Sweep".into(),
                parameters: vec![SweepParameter {
                    name: "dose".into(),
                    values: vec![
                        ParameterValue::Int(1),
                        ParameterValue::Int(2),
                        ParameterValue::Int(3),
                    ],
                }],
                strategy: composure_core::SweepStrategy::Random,
                sample_count: Some(6),
                seed: Some(7),
                metadata: None,
            },
            bundle: None,
            executed_cases: vec![composure_core::ExecutedSweepCase {
                case: SweepCase {
                    case_id: "sweep-1-1".into(),
                    parameters: parameters.clone(),
                },
                parameter_set: ExperimentParameterSet::new(
                    "variant-a",
                    "Variant A",
                    Scenario::new(
                        "variant-a",
                        "Variant A",
                        composure_core::SimState::zeros(1),
                        5,
                    ),
                ),
                run: composure_core::ExperimentRunRecord::running(
                    "run-1",
                    Some("variant-a"),
                    Some(7),
                )
                .mark_completed(composure_core::ExperimentOutcome {
                    monte_carlo: None,
                    composure: None,
                    replay: None,
                    metadata: None,
                }),
                summary: RunSummary {
                    monte_carlo: None,
                    composure: None,
                },
                sample: Some(composure_core::SweepSample {
                    case_id: "sweep-1-1".into(),
                    parameters: parameters.clone(),
                    objective: 0.69,
                    metadata: Some(serde_json::json!({
                        "run_id": "run-1",
                        "parameter_set_id": "variant-a"
                    })),
                }),
            }],
            failures: vec![],
            samples: vec![composure_core::SweepSample {
                case_id: "sweep-1-1".into(),
                parameters,
                objective: 0.69,
                metadata: Some(serde_json::json!({
                    "run_id": "run-1",
                    "parameter_set_id": "variant-a"
                })),
            }],
            sensitivity: Some(composure_core::SensitivityReport {
                sample_count: 3,
                objective: composure_core::ObjectiveSummary {
                    min: 0.69,
                    max: 0.69,
                    mean: 0.69,
                    best_case_id: "sweep-1-1".into(),
                    worst_case_id: "sweep-1-1".into(),
                },
                rankings: vec![composure_core::ParameterSensitivity {
                    parameter: "dose".into(),
                    score: 0.9,
                    direction: SensitivityDirection::Positive,
                    kind: SensitivityKind::Numeric(composure_core::NumericSensitivityStats {
                        correlation: 0.9,
                        slope: 0.5,
                    }),
                }],
                config: SensitivityConfig::default(),
            }),
            config: SweepRunnerConfig {
                run_id_prefix: "run".into(),
                execution: ExperimentExecutionConfig::default(),
                sensitivity: SensitivityConfig::default(),
                failure_mode: SweepFailureMode::Continue,
            },
        }
    }

    #[test]
    fn test_format_bundle() {
        let output = format_bundle(&sample_bundle());
        assert!(output.contains("Bundle: Baseline (exp-1)"));
        assert!(output.contains("Parameter sets: 1"));
        assert!(output.contains("completed=1"));
    }

    #[test]
    fn test_render_bundle_markdown() {
        let output = render_bundle_markdown(&sample_bundle());
        assert!(output.contains("# Experiment Bundle: Baseline (exp-1)"));
        assert!(output.contains("- Default Monte Carlo: `10 paths / 5 steps / seed 42`"));
        assert!(output.contains("## Parameter Sets"));
        assert!(output.contains("| variant-a | Variant A | 5 | 10 paths / 5 steps / seed 7 |"));
        assert!(output.contains("## Runs"));
        assert!(output.contains("| run-1 | Completed | variant-a | 7 | no | no |  |"));
    }

    #[test]
    fn test_format_sweep() {
        let output = format_sweep(&sample_sweep_result());

        assert!(output.contains("Strategy: Random"));
        assert!(output.contains("Top sensitivity: dose"));
    }

    #[test]
    fn test_render_sweep_summary_markdown() {
        let output = render_sweep_summary_markdown(&sample_sweep_result());
        assert!(output.contains("# Sweep Summary: Sweep (sweep-1)"));
        assert!(output.contains("| Best Case | Best Objective | Worst Case | Mean Objective |"));
        assert!(output.contains("| sweep-1-1 | 0.6900 | sweep-1-1 | 0.6900 |"));
        assert!(output.contains("| sweep-1-1 | 0.6900 | run-1 | variant-a | dose=2 |"));
        assert!(output.contains("| dose | Positive | 0.9000 | numeric corr=0.9000, slope=0.5000 |"));
    }

    #[test]
    fn test_format_summary() {
        let output = format_summary(&RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 5,
                num_paths: 100,
                start: Some(0.8),
                end: Some(0.4),
                min: Some(0.4),
                max: Some(0.8),
                mean: Some(0.6),
                auc: Some(2.4),
                p10_end: Some(0.3),
                p50_end: Some(0.4),
                p90_end: Some(0.5),
                final_band_width: Some(0.2),
            }),
            composure: Some(ComposureSummary {
                archetype: composure_core::Archetype::Phoenix,
                slope: -0.2,
                variance: 0.1,
                peak: 0.9,
                trough: 0.3,
                recovery_half_life: Some(2),
                residual_damage: 0.1,
                break_point: Some(1),
            }),
        });

        assert!(output.contains("Monte Carlo: present"));
        assert!(output.contains("final mean: Some(0.4)"));
        assert!(output.contains("Composure: present"));
        assert!(output.contains("archetype: Phoenix"));
    }

    #[test]
    fn test_format_report() {
        let output = format_report(&sample_report());
        assert!(output.contains("Start delta:"));
        assert!(output.contains("Archetype change:"));
        assert!(output.contains("Phoenix"));
        assert!(output.contains("Comparison: rmse="));
    }

    #[test]
    fn test_render_report_markdown() {
        let output = render_report_markdown(&sample_report());
        assert!(output.contains("# Deterministic Report"));
        assert!(output.contains("| Start | 0.8400 | 0.8600 |"));
        assert!(output.contains("## Comparison Snapshot"));
    }

    #[test]
    fn test_format_calibration() {
        let output = format_calibration(&sample_calibration_result());
        assert!(output.contains("Calibration: Dose Sweep (dose-sweep)"));
        assert!(output.contains("Observed: Observed Recovery (obs-1)"));
        assert!(output.contains("Failures: 1"));
        assert!(output.contains("Top candidate: case=dose-sweep-1"));
    }

    #[test]
    fn test_render_sweep_csv() {
        let output = render_sweep_csv(&sample_sweep_result());
        assert!(output.contains("case_id,is_best,objective,run_id,parameter_set_id,dose"));
        assert!(output.contains("sweep-1-1,true,0.690000000000,run-1,variant-a,2"));
    }

    #[test]
    fn test_render_sweep_markdown() {
        let output = render_sweep_markdown(&sample_sweep_result());
        assert!(output.contains("# Sweep Samples: Sweep (sweep-1)"));
        assert!(output.contains("| Case | Best | Objective | Run | Parameter Set | dose |"));
        assert!(output.contains("| sweep-1-1 | yes | 0.6900 | run-1 | variant-a | 2 |"));
    }

    #[test]
    fn test_render_calibration_csv() {
        let output = render_calibration_csv(&sample_calibration_result());
        assert!(output.contains("rank,is_best,case_id,parameter_set_id,run_id,score"));
        assert!(output.contains("1,true,dose-sweep-1,ps-dose-sweep-1,calibration-run-1"));
        assert!(output.contains(",3"));
    }

    #[test]
    fn test_render_calibration_markdown() {
        let output = render_calibration_markdown(&sample_calibration_result());
        assert!(output.contains("# Calibration Candidates: Dose Sweep (dose-sweep)"));
        assert!(output.contains("| Rank | Best | Case | Parameter Set | Run | Score | RMSE | Mean Abs Delta | End Delta | dose |"));
        assert!(output.contains("| 1 | yes | dose-sweep-1 | ps-dose-sweep-1 | calibration-run-1 | 0.0356 | 0.0356 | 0.0333 | 0.0500 | 3 |"));
    }

    #[test]
    fn test_run_help() {
        let output = run(&["composure".into(), "help".into()]).unwrap();
        assert!(output.contains("inspect-pack"));
        assert!(output.contains("inspect-counterfactual"));
        assert!(output.contains("validate-pack"));
        assert!(output.contains("validate-counterfactual"));
        assert!(output.contains("run-pack"));
        assert!(output.contains("run-counterfactual"));
        assert!(output.contains("inspect-report"));
        assert!(output.contains("inspect-calibration"));
        assert!(output.contains("export-bundle-markdown"));
        assert!(output.contains("export-report-markdown"));
        assert!(output.contains("export-sweep-summary-markdown"));
        assert!(output.contains("export-sweep-samples"));
        assert!(output.contains("export-sweep-samples-markdown"));
        assert!(output.contains("export-calibration-candidates"));
        assert!(output.contains("export-calibration-candidates-markdown"));
        assert!(output.contains("summarize-bundle-run"));
        assert!(output.contains("build-report"));
        assert!(output.contains("simulate-marketing"));
        assert!(output.contains("simulate-marketing-v2"));
        assert!(output.contains("simulate-marketing-v2-assisted"));
        assert!(output.contains("compare-marketing-v2-assisted"));
        assert!(output.contains("export-marketing-v2-report-markdown"));
        assert!(output.contains("export-marketing-v2-compare-markdown"));
        assert!(output.contains("--provider <name>"));
        assert!(output.contains("--model <name>"));
        assert!(output.contains("--reasoning-effort <level>"));
        assert!(output.contains("--output <path>"));
    }

    #[test]
    fn test_run_simulate_marketing_v2_recognizes_command() {
        let temp_dir = std::env::temp_dir();
        let request_path = temp_dir.join("composure-cli-marketing-v2.json");
        fs::write(
            &request_path,
            serde_json::json!({
                "project": {
                    "name": "Composure",
                    "description": "Deterministic simulation for campaigns",
                    "platform_context": ["twitter", "linkedin"]
                },
                "personas": [
                    {
                        "id": "dev",
                        "name": "Pragmatic Dev",
                        "type": "developer",
                        "relationship": "existing customer",
                        "preferences": ["practical examples", "clear frameworks"],
                        "objections": ["vague marketing", "tool sprawl"]
                    }
                ],
                "approaches": [
                    {
                        "id": "specific",
                        "angle": "Show founders how to rank hooks before publishing",
                        "format": "Twitter thread",
                        "channels": ["twitter"],
                        "tone": "direct and contrarian",
                        "target": "technical founders"
                    }
                ],
                "simulation_size": 8
            })
            .to_string(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "simulate-marketing-v2".into(),
            request_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("\"approach_results\""));

        let _ = fs::remove_file(request_path);
    }

    #[test]
    fn test_run_simulate_marketing_v2_assisted_can_skip_llm() {
        let temp_dir = std::env::temp_dir();
        let request_path = temp_dir.join("composure-cli-marketing-v2-assisted.json");
        fs::write(
            &request_path,
            serde_json::json!({
                "project": {
                    "name": "Composure",
                    "description": "Deterministic simulation for campaigns",
                    "platform_context": ["twitter", "linkedin"]
                },
                "personas": [
                    {
                        "id": "dev",
                        "name": "Pragmatic Dev",
                        "type": "developer",
                        "relationship": "existing customer",
                        "preferences": ["practical examples", "clear frameworks"],
                        "objections": ["vague marketing", "tool sprawl"]
                    }
                ],
                "approaches": [
                    {
                        "id": "specific",
                        "angle": "Show founders how to rank hooks before publishing",
                        "format": "Twitter thread",
                        "channels": ["twitter"],
                        "tone": "direct and contrarian",
                        "target": "technical founders"
                    }
                ],
                "llm_assist": {
                    "enabled": false
                },
                "simulation_size": 8
            })
            .to_string(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "simulate-marketing-v2-assisted".into(),
            request_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("\"approach_results\""));

        let _ = fs::remove_file(request_path);
    }

    #[test]
    fn test_run_simulate_marketing_v2_assisted_applies_cli_evaluator_overrides() {
        let temp_dir = std::env::temp_dir();
        let request_path = temp_dir.join("composure-cli-marketing-v2-assisted-with-overrides.json");
        fs::write(
            &request_path,
            serde_json::json!({
                "project": {
                    "name": "Composure",
                    "description": "Deterministic simulation for campaigns",
                    "platform_context": ["twitter", "linkedin"]
                },
                "personas": [
                    {
                        "id": "dev",
                        "name": "Pragmatic Dev",
                        "type": "developer",
                        "relationship": "existing customer",
                        "preferences": ["practical examples", "clear frameworks"],
                        "objections": ["vague marketing", "tool sprawl"]
                    }
                ],
                "approaches": [
                    {
                        "id": "specific",
                        "angle": "Show founders how to rank hooks before publishing",
                        "format": "Twitter thread",
                        "channels": ["twitter"],
                        "tone": "direct and contrarian",
                        "target": "technical founders"
                    }
                ],
                "llm_assist": {
                    "enabled": false
                },
                "simulation_size": 8
            })
            .to_string(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "simulate-marketing-v2-assisted".into(),
            request_path.display().to_string(),
            "--provider".into(),
            "cliproxyapi".into(),
            "--model".into(),
            "gpt-5.4".into(),
            "--reasoning-effort".into(),
            "high".into(),
        ])
        .unwrap();

        let parsed: MarketingSimulationResultV2 = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.engine.provider.as_deref(), Some("cliproxyapi"));
        assert_eq!(parsed.engine.model, "gpt-5.4");
        assert_eq!(parsed.engine.reasoning_effort.as_deref(), Some("high"));

        let _ = fs::remove_file(request_path);
    }

    #[test]
    fn test_render_marketing_v2_report_markdown() {
        let request = serde_json::json!({
            "project": {
                "name": "Composure",
                "description": "Deterministic simulation for campaigns",
                "platform_context": ["landing page", "tiktok"]
            },
            "personas": [
                {
                    "id": "builder",
                    "name": "Builder",
                    "type": "founder",
                    "jobs": ["ship faster"],
                    "preferences": ["proof"],
                    "trust_signals": ["proof"],
                    "channels": ["landing page"]
                },
                {
                    "id": "operator",
                    "name": "Operator",
                    "type": "operator",
                    "jobs": ["share what works"],
                    "preferences": ["community"],
                    "channels": ["tiktok"]
                }
            ],
            "approaches": [
                {
                    "id": "lp-proof",
                    "angle": "Proof on the landing page",
                    "format": "landing page headline",
                    "channels": ["landing page"],
                    "tone": "clear",
                    "target": "founders",
                    "proof_points": ["case study"],
                    "objection_handlers": ["no hype"],
                    "cta": "Join waitlist"
                },
                {
                    "id": "tt-community",
                    "angle": "Show the operator community sharing wins",
                    "format": "short-form video",
                    "channels": ["tiktok", "community"],
                    "tone": "energetic",
                    "target": "operators",
                    "sequence": [
                        { "label": "hook", "focus": "attention", "intensity": 1.2 },
                        { "label": "proof", "focus": "resonance", "intensity": 1.0 },
                        { "label": "cta", "focus": "conversion", "intensity": 1.0 }
                    ]
                }
            ],
            "scenario": {
                "name": "short form",
                "scenario_type": "short_form_video",
                "time_steps": 6
            },
            "simulation_size": 8
        });

        let mut result = simulate_marketing_v2(&serde_json::from_value(request).unwrap()).unwrap();
        result.llm_analysis = Some(composure_marketing::MarketingLlmAnalysis {
            provider: Some("openai".into()),
            model: "gpt-test".into(),
            reasoning_effort: Some("medium".into()),
            evaluator_count: 1,
            executive_summary: vec!["One clear winner emerged.".into()],
            consensus_summary: vec![],
            strategic_takeaways: vec!["Keep the proof-forward angle.".into()],
            recommended_next_experiments: vec!["Test a sharper CTA.".into()],
            confidence_notes: vec!["Audit metadata was captured.".into()],
            disagreement_notes: vec![],
            evidence: None,
        });
        result.llm_trace = Some(composure_marketing::MarketingLlmTrace {
            analysis_goal: Some("stress test realism".into()),
            system_prompt: "System prompt".into(),
            user_prompt: "User prompt".into(),
            prompt_char_count: 22,
            evaluators: vec![composure_marketing::LlmEvaluatorTrace {
                evaluator_index: 1,
                provider: Some("openai".into()),
                model: "gpt-test".into(),
                reasoning_effort: Some("medium".into()),
                base_url: "https://api.openai.com/v1".into(),
                requested_max_output_tokens: Some(512),
                stream_fallback_used: true,
                duration_ms: 321,
                response_id: Some("resp_123".into()),
                usage: Some(composure_marketing::LlmUsage {
                    input_tokens: Some(120),
                    output_tokens: Some(60),
                    reasoning_tokens: Some(10),
                    total_tokens: Some(180),
                }),
                raw_response: serde_json::json!({
                    "raw_stream_text": "data: example"
                }),
                raw_output_text: "{\"executive_summary\":[]}".into(),
                parsed_output: Some(serde_json::json!({
                    "executive_summary": [],
                    "strategic_takeaways": [],
                    "recommended_next_experiments": [],
                    "confidence_notes": [],
                    "approach_analyses": []
                })),
            }],
        });
        let output = render_marketing_v2_report_markdown(&result);

        assert!(output.contains("Persona Leaderboard"));
        assert!(output.contains("Repeated Concerns"));
        assert!(output.contains("Confidence Notes"));
        assert!(output.contains("Recommended Next Experiments"));
        assert!(output.contains("LLM Evidence"));
        assert!(output.contains("Stream fallback used"));
    }

    #[test]
    fn test_run_export_marketing_v2_report_markdown() {
        let temp_dir = std::env::temp_dir();
        let request_path = temp_dir.join("composure-cli-marketing-v2-report-request.json");
        let artifact_path = temp_dir.join("composure-cli-marketing-v2-report.json");

        fs::write(
            &request_path,
            serde_json::json!({
                "project": {
                    "name": "Composure",
                    "description": "Deterministic simulation for campaigns",
                    "platform_context": ["landing page"]
                },
                "personas": [
                    {
                        "id": "builder",
                        "name": "Builder",
                        "type": "founder",
                        "jobs": ["ship faster"],
                        "preferences": ["proof"]
                    }
                ],
                "approaches": [
                    {
                        "id": "lp-proof",
                        "angle": "Proof on the landing page",
                        "format": "landing page headline",
                        "channels": ["landing page"],
                        "tone": "clear",
                        "target": "founders",
                        "cta": "Join waitlist"
                    }
                ],
                "scenario": {
                    "name": "landing",
                    "scenario_type": "landing_page",
                    "time_steps": 6
                },
                "simulation_size": 8
            })
            .to_string(),
        )
        .unwrap();

        let raw = run(&[
            "composure".into(),
            "simulate-marketing-v2".into(),
            request_path.display().to_string(),
            "--output".into(),
            artifact_path.display().to_string(),
        ])
        .unwrap();
        assert!(raw.contains("Wrote artifact"));

        let markdown = run(&[
            "composure".into(),
            "export-marketing-v2-report-markdown".into(),
            artifact_path.display().to_string(),
        ])
        .unwrap();

        assert!(markdown.contains("Marketing Simulation Report"));
        assert!(markdown.contains("Recommended Next Experiments"));

        let _ = fs::remove_file(request_path);
        let _ = fs::remove_file(artifact_path);
    }

    #[test]
    fn test_compare_marketing_v2_assisted_requires_two_paths() {
        let err = run(&[
            "composure".into(),
            "compare-marketing-v2-assisted".into(),
            "only-one.json".into(),
        ])
        .unwrap_err();

        assert!(err.to_string().contains("at least two request paths"));
    }

    #[test]
    fn test_run_compare_marketing_v2_assisted_and_export_markdown() {
        let temp_dir = std::env::temp_dir();
        let request_a = temp_dir.join("compare-marketing-a.json");
        let request_b = temp_dir.join("compare-marketing-b.json");
        let artifact = temp_dir.join("compare-marketing-artifact.json");
        fs::write(
            &request_a,
            serde_json::json!({
                "project": {
                    "name": "Composure",
                    "description": "Deterministic simulation for campaigns",
                    "platform_context": ["landing page"]
                },
                "personas": [{
                    "id": "builder",
                    "name": "Builder",
                    "type": "founder",
                    "jobs": ["ship faster"],
                    "preferences": ["proof"],
                    "trust_signals": ["proof"],
                    "channels": ["landing page"]
                }],
                "approaches": [{
                    "id": "lp-proof",
                    "angle": "Proof on the landing page",
                    "format": "landing page headline",
                    "channels": ["landing page"],
                    "tone": "clear",
                    "target": "founders",
                    "proof_points": ["case study"],
                    "objection_handlers": ["no hype"],
                    "cta": "Join waitlist"
                }],
                "scenario": {
                    "name": "Landing",
                    "scenario_type": "landing_page",
                    "time_steps": 6
                },
                "llm_assist": {
                    "enabled": false
                },
                "simulation_size": 8
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            &request_b,
            serde_json::json!({
                "project": {
                    "name": "Composure",
                    "description": "Deterministic simulation for campaigns",
                    "platform_context": ["tiktok"]
                },
                "personas": [{
                    "id": "operator",
                    "name": "Operator",
                    "type": "operator",
                    "jobs": ["share what works"],
                    "preferences": ["community"],
                    "channels": ["tiktok"]
                }],
                "approaches": [{
                    "id": "tt-community",
                    "angle": "Show the operator community sharing wins",
                    "format": "short-form video",
                    "channels": ["tiktok", "community"],
                    "tone": "energetic",
                    "target": "operators",
                    "sequence": [
                        { "label": "hook", "focus": "attention", "intensity": 1.2 },
                        { "label": "proof", "focus": "resonance", "intensity": 1.0 },
                        { "label": "cta", "focus": "conversion", "intensity": 1.0 }
                    ]
                }],
                "scenario": {
                    "name": "Short form",
                    "scenario_type": "short_form_video",
                    "time_steps": 6
                },
                "llm_assist": {
                    "enabled": false
                },
                "simulation_size": 8
            })
            .to_string(),
        )
        .unwrap();

        let raw = run(&[
            "composure".into(),
            "compare-marketing-v2-assisted".into(),
            request_a.display().to_string(),
            request_b.display().to_string(),
            "--output".into(),
            artifact.display().to_string(),
        ])
        .unwrap();
        assert!(raw.contains("Wrote artifact"));

        let report =
            read_json::<MarketingV2ComparisonReport>(&artifact.display().to_string()).unwrap();
        assert_eq!(report.scenarios.len(), 2);

        let short_form = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_name == "Short form")
            .unwrap();
        let landing = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_name == "Landing")
            .unwrap();
        assert_eq!(short_form.scenario_type, "short_form_video");
        assert_eq!(landing.scenario_type, "landing_page");

        assert_eq!(
            short_form.strongest_positive_delta_metric.as_deref(),
            Some("Belonging")
        );
        assert_eq!(short_form.strongest_positive_delta_value, Some(22));
        assert_eq!(
            short_form.weakest_delta_metric.as_deref(),
            Some("Objection Pressure")
        );
        assert_eq!(short_form.weakest_delta_value, Some(-11));
        assert_eq!(
            landing.strongest_positive_delta_metric.as_deref(),
            Some("Objection Pressure")
        );
        assert_eq!(landing.strongest_positive_delta_value, Some(11));
        assert_eq!(landing.weakest_delta_metric.as_deref(), Some("Belonging"));
        assert_eq!(landing.weakest_delta_value, Some(-22));

        let short_form_belonging = short_form
            .metric_deltas
            .iter()
            .find(|metric| metric.label == "Belonging")
            .unwrap();
        assert_eq!(short_form_belonging.score, 84);
        assert_eq!(short_form_belonging.delta_vs_compare_average, 22);
        assert_eq!(short_form_belonging.delta_vs_compare_leader, 0);
        assert_eq!(short_form_belonging.compare_set_rank, 1);
        assert_eq!(short_form_belonging.compare_set_size, 2);
        assert_eq!(short_form_belonging.leading_scenarios, vec!["Short form"]);

        let landing_objection_pressure = landing
            .metric_deltas
            .iter()
            .find(|metric| metric.label == "Objection Pressure")
            .unwrap();
        assert_eq!(landing_objection_pressure.score, 85);
        assert_eq!(landing_objection_pressure.delta_vs_compare_average, 11);
        assert_eq!(landing_objection_pressure.delta_vs_compare_leader, 0);
        assert_eq!(landing_objection_pressure.compare_set_rank, 1);
        assert_eq!(landing_objection_pressure.compare_set_size, 2);
        assert_eq!(
            landing_objection_pressure.leading_scenarios,
            vec!["Landing"]
        );

        let landing_belonging = landing
            .metric_deltas
            .iter()
            .find(|metric| metric.label == "Belonging")
            .unwrap();
        assert_eq!(landing_belonging.score, 40);
        assert_eq!(landing_belonging.delta_vs_compare_average, -22);
        assert_eq!(landing_belonging.delta_vs_compare_leader, -44);
        assert_eq!(landing_belonging.compare_set_rank, 2);
        assert_eq!(landing_belonging.compare_set_size, 2);
        assert_eq!(landing_belonging.leading_scenarios, vec!["Short form"]);

        let markdown = run(&[
            "composure".into(),
            "export-marketing-v2-compare-markdown".into(),
            artifact.display().to_string(),
        ])
        .unwrap();
        assert!(markdown.contains("Marketing V2 Comparison"));
        assert!(markdown.contains("Leaderboard"));
        assert!(markdown.contains("Cross-Scenario Metric Delta Matrix"));
        assert!(markdown.contains("aggregate primary scorecard"));
        assert!(markdown.contains("Metric Delta Leaders"));
        assert!(markdown.contains("Scenario Notes"));
        assert!(markdown.contains("Portfolio Recommendation"));
        assert!(markdown.contains("Strongest Metric"));
        assert!(markdown.contains("Metric deltas vs compare set"));
        assert!(markdown.contains("Vs Leader"));
        assert!(markdown.contains("Leader(s)"));
        assert!(markdown.contains("| Belonging | 44 | +22 | -22 |"));
        assert!(markdown.contains("| Belonging | Short form (+22) | Landing (-22) |"));
        assert!(markdown
            .contains("- Strongest cross-scenario delta: `Belonging` at `+22` vs compare average"));
        assert!(markdown
            .contains("- Weakest cross-scenario delta: `Belonging` at `-22` vs compare average"));
        assert!(markdown.contains("| Belonging | 84 | +22 | +0 | 1/2 | Short form |"));
        assert!(markdown.contains("| Objection Pressure | 85 | +11 | +0 | 1/2 | Landing |"));

        let _ = fs::remove_file(request_a);
        let _ = fs::remove_file(request_b);
        let _ = fs::remove_file(artifact);
    }

    #[test]
    fn test_merge_common_option_tracks_consensus_and_conflicts() {
        let mut slot = None;
        merge_common_option(&mut slot, Some("gpt-5.4".to_string()));
        assert_eq!(slot, Some(Some("gpt-5.4".to_string())));

        merge_common_option(&mut slot, Some("gpt-5.4".to_string()));
        assert_eq!(slot, Some(Some("gpt-5.4".to_string())));

        merge_common_option(&mut slot, Some("gpt-5.3-codex".to_string()));
        assert_eq!(slot, Some(None));

        merge_common_option(&mut slot, Some("gpt-5.4".to_string()));
        assert_eq!(slot, Some(None));
    }

    #[test]
    fn test_format_comparison() {
        let comparison = composure_core::compare_trajectories(
            &[0.9, 0.8, 0.7],
            &[0.9, 0.82, 0.6],
            &composure_core::ComparisonConfig {
                failure_threshold: Some(0.65),
                ..composure_core::ComparisonConfig::default()
            },
        )
        .unwrap();

        let output = format_comparison(&comparison);
        assert!(output.contains("Series length: 3"));
        assert!(output.contains("Failure comparison:"));
    }

    #[test]
    fn test_parse_compare_options_overrides_defaults() {
        let options = parse_compare_options(&[
            "--divergence-threshold".into(),
            "0.2".into(),
            "--sustained-steps".into(),
            "3".into(),
            "--equality-epsilon".into(),
            "0.01".into(),
            "--failure-threshold".into(),
            "0.4".into(),
        ])
        .unwrap();

        assert_eq!(options.config.divergence_threshold, 0.2);
        assert_eq!(options.config.sustained_steps, 3);
        assert_eq!(options.config.equality_epsilon, 0.01);
        assert_eq!(options.config.failure_threshold, Some(0.4));
        assert!(options.output_path.is_none());
    }

    #[test]
    fn test_parse_compare_options_accepts_output() {
        let options = parse_compare_options(&[
            "--divergence-threshold".into(),
            "0.2".into(),
            "--output".into(),
            "/tmp/comparison.json".into(),
        ])
        .unwrap();

        assert_eq!(options.config.divergence_threshold, 0.2);
        assert_eq!(options.output_path.as_deref(), Some("/tmp/comparison.json"));
    }

    #[test]
    fn test_parse_compare_options_rejects_unknown_flag() {
        let err = parse_compare_options(&["--unknown".into(), "1".into()]).unwrap_err();
        assert!(matches!(err, CliError::UnknownFlag(flag) if flag == "--unknown"));
    }

    #[test]
    fn test_parse_build_report_options_accepts_flags() {
        let options = parse_build_report_options(&[
            "--comparison".into(),
            "/tmp/comparison.json".into(),
            "--output".into(),
            "/tmp/report.json".into(),
        ])
        .unwrap();

        assert_eq!(
            options.comparison_path.as_deref(),
            Some("/tmp/comparison.json")
        );
        assert_eq!(options.output_path.as_deref(), Some("/tmp/report.json"));
    }

    #[test]
    fn test_parse_build_report_options_rejects_unknown_flag() {
        let err = parse_build_report_options(&["--unknown".into(), "1".into()]).unwrap_err();
        assert!(matches!(err, CliError::UnknownFlag(flag) if flag == "--unknown"));
    }

    #[test]
    fn test_run_compare_monte_carlo_outputs_json() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline.json");
        let candidate_path = temp_dir.join("composure-cli-candidate.json");

        let baseline = sample_monte_carlo(1, [0.84, 0.74, 0.64]);
        let candidate = sample_monte_carlo(2, [0.86, 0.77, 0.69]);

        fs::write(&baseline_path, serde_json::to_string(&baseline).unwrap()).unwrap();
        fs::write(&candidate_path, serde_json::to_string(&candidate).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "compare-monte-carlo".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
        ])
        .unwrap();

        let comparison: TrajectoryComparison = serde_json::from_str(&output).unwrap();
        assert_eq!(comparison.series_len, 3);
        assert!(comparison.metrics.mean_delta > 0.0);

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
    }

    #[test]
    fn test_run_compare_monte_carlo_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline-output.json");
        let candidate_path = temp_dir.join("composure-cli-candidate-output.json");
        let output_path = temp_dir.join("composure-cli-comparison-output.json");

        fs::write(
            &baseline_path,
            serde_json::to_string(&sample_monte_carlo(1, [0.84, 0.74, 0.64])).unwrap(),
        )
        .unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_string(&sample_monte_carlo(2, [0.86, 0.77, 0.69])).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "compare-monte-carlo".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let comparison: TrajectoryComparison = serde_json::from_str(&written).unwrap();
        assert_eq!(comparison.series_len, 3);

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_build_report_outputs_json() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline-summary.json");
        let candidate_path = temp_dir.join("composure-cli-candidate-summary.json");
        let comparison_path = temp_dir.join("composure-cli-build-report-comparison.json");

        fs::write(
            &baseline_path,
            serde_json::to_string(&sample_baseline_summary()).unwrap(),
        )
        .unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_string(&sample_candidate_summary()).unwrap(),
        )
        .unwrap();
        fs::write(
            &comparison_path,
            serde_json::to_string(&sample_comparison()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "build-report".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
            "--comparison".into(),
            comparison_path.display().to_string(),
        ])
        .unwrap();

        let report: DeterministicReport = serde_json::from_str(&output).unwrap();
        assert_eq!(report.end_delta.delta, Some(0.04999999999999993));
        assert!(report.comparison.is_some());

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
        let _ = fs::remove_file(comparison_path);
    }

    #[test]
    fn test_run_build_report_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline-summary-output.json");
        let candidate_path = temp_dir.join("composure-cli-candidate-summary-output.json");
        let output_path = temp_dir.join("composure-cli-report-output.json");

        fs::write(
            &baseline_path,
            serde_json::to_string(&sample_baseline_summary()).unwrap(),
        )
        .unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_string(&sample_candidate_summary()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "build-report".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let report: DeterministicReport = serde_json::from_str(&written).unwrap();
        assert!(report.comparison.is_none());

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_report_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-report-markdown-input.json");
        let output_path = temp_dir.join("composure-cli-report-markdown-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_report()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-report-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Deterministic Report"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_synthetic_market_report_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-synthetic-market-input.json");
        let output_path = temp_dir.join("composure-cli-synthetic-market-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_synthetic_market_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-synthetic-market-report-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Synthetic Market Report"));
        assert!(written.contains("`peptide_glp1_wedge`"));
        assert!(written.contains("Organic readiness"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    fn temp_pack_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("composure-cli-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_sample_pack(dir: &std::path::Path) {
        let mut scenario = Scenario::new(
            "pack-scenario",
            "Pack Scenario",
            composure_core::SimState::zeros(2),
            4,
        );
        scenario.initial_state =
            composure_core::SimState::new(vec![0.4, 0.5], vec![0.1, 0.2], vec![0.2, 0.2]);
        scenario.failure_threshold = Some(0.45);
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "stabilize-sleep".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.45 },
            action: Action {
                dimension: Some(0),
                magnitude: 0.1,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 1,
            cooldown_steps: 2,
            priority: 1,
            max_fires: Some(1),
        });
        scenario.metadata = Some(serde_json::json!({
            "dimension_labels": ["sleep", "readiness"]
        }));

        let mut spec = ExperimentSpec::new("pack-exp", "Pack Experiment", scenario.clone());
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(8, 4, 7));

        let observed = ObservedTrajectory::new("obs-1", "Observed", vec![0.4, 0.45, 0.5, 0.55]);

        let mut sweep = SweepDefinition::new("sweep-1", "Sweep");
        sweep.parameters.push(SweepParameter {
            name: "dose".into(),
            values: vec![ParameterValue::Int(1)],
        });

        fs::write(
            dir.join("scenario.json"),
            serde_json::to_string_pretty(&scenario).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("experiment-spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("sweep-definition.json"),
            serde_json::to_string_pretty(&sweep).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("observed-trajectory.json"),
            serde_json::to_string_pretty(&observed).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "id": "health-pack",
                "name": "Health Pack",
                "scenario": "scenario.json",
                "experiment_spec": "experiment-spec.json",
                "sweep_definition": "sweep-definition.json",
                "observed_trajectory": "observed-trajectory.json",
                "runtime_model": {
                    "kind": "linear",
                    "dimensions": [
                        {
                            "drift": 0.01,
                            "action_gain": 0.08,
                            "memory_decay": 0.1,
                            "action_to_memory": 0.06,
                            "memory_to_state": 0.04,
                            "uncertainty_decay": 0.05,
                            "action_to_uncertainty": 0.2,
                            "min_value": 0.0,
                            "max_value": 1.0
                        },
                        {
                            "drift": 0.015,
                            "action_gain": 0.06,
                            "memory_decay": 0.08,
                            "action_to_memory": 0.05,
                            "memory_to_state": 0.03,
                            "uncertainty_decay": 0.05,
                            "action_to_uncertainty": 0.2,
                            "min_value": 0.0,
                            "max_value": 1.0
                        }
                    ],
                    "action_type_scales": {
                        "intervention": 1.0,
                        "stressor_onset": 1.0,
                        "stressor_removal": 0.8,
                        "hold": 0.0,
                        "custom": {}
                    },
                    "noise_scale": 0.01,
                    "aggregate_weights": [0.7, 0.3]
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn write_sample_counterfactual(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("counterfactual.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::json!({
                "id": "cf-1",
                "name": "Recovery branch",
                "branch_state": {
                    "z": [0.4, 0.5],
                    "m": [0.1, 0.2],
                    "u": [0.2, 0.2],
                    "t": 3
                },
                "baseline": {
                    "branch_id": "baseline",
                    "intervention_label": "No change",
                    "actions": [
                        {
                            "dimension": 0,
                            "magnitude": 0.0,
                            "action_type": "Hold",
                            "metadata": null
                        },
                        {
                            "dimension": 0,
                            "magnitude": 0.0,
                            "action_type": "Hold",
                            "metadata": null
                        },
                        {
                            "dimension": 0,
                            "magnitude": 0.0,
                            "action_type": "Hold",
                            "metadata": null
                        },
                        {
                            "dimension": 0,
                            "magnitude": 0.0,
                            "action_type": "Hold",
                            "metadata": null
                        }
                    ],
                    "conditional_actions": []
                },
                "candidate": {
                    "branch_id": "candidate",
                    "intervention_label": "Recovery",
                    "actions": [
                        {
                            "dimension": 0,
                            "magnitude": 0.2,
                            "action_type": "Intervention",
                            "metadata": null
                        },
                        {
                            "dimension": 0,
                            "magnitude": 0.2,
                            "action_type": "Intervention",
                            "metadata": null
                        },
                        {
                            "dimension": 0,
                            "magnitude": 0.2,
                            "action_type": "Intervention",
                            "metadata": null
                        },
                        {
                            "dimension": 0,
                            "magnitude": 0.2,
                            "action_type": "Intervention",
                            "metadata": null
                        }
                    ],
                    "conditional_actions": [
                        {
                            "id": "stabilize",
                            "trigger": {
                                "kind": "health_index_below",
                                "threshold": 0.45
                            },
                            "action": {
                                "dimension": 1,
                                "magnitude": 0.1,
                                "action_type": "Intervention",
                                "metadata": null
                            },
                            "delay_steps": 1,
                            "cooldown_steps": 2,
                            "priority": 1,
                            "max_fires": 1
                        }
                    ]
                },
                "config": {
                    "monte_carlo": {
                        "num_paths": 6,
                        "time_steps": 4,
                        "seed_base": 19
                    },
                    "execution": {
                        "retain_paths": true,
                        "analyze_composure": true
                    },
                    "comparison": {
                        "divergence_threshold": 0.1,
                        "sustained_steps": 1,
                        "equality_epsilon": 0.000000001,
                        "failure_threshold": 0.45
                    },
                    "analysis_failure_threshold": 0.45
                },
                "runtime_model": {
                    "kind": "linear",
                    "dimensions": [
                        {
                            "drift": 0.01,
                            "action_gain": 0.08,
                            "memory_decay": 0.1,
                            "action_to_memory": 0.06,
                            "memory_to_state": 0.04,
                            "uncertainty_decay": 0.05,
                            "action_to_uncertainty": 0.2,
                            "min_value": 0.0,
                            "max_value": 1.0
                        },
                        {
                            "drift": 0.015,
                            "action_gain": 0.06,
                            "memory_decay": 0.08,
                            "action_to_memory": 0.05,
                            "memory_to_state": 0.03,
                            "uncertainty_decay": 0.05,
                            "action_to_uncertainty": 0.2,
                            "min_value": 0.0,
                            "max_value": 1.0
                        }
                    ],
                    "action_type_scales": {
                        "intervention": 1.0,
                        "stressor_onset": 1.0,
                        "stressor_removal": 0.8,
                        "hold": 0.0,
                        "custom": {}
                    },
                    "noise_scale": 0.01,
                    "aggregate_weights": [0.7, 0.3]
                }
            }))
            .unwrap(),
        )
        .unwrap();
        path
    }

    fn write_sample_pack_with_counterfactual(dir: &std::path::Path) {
        write_sample_pack(dir);
        write_sample_counterfactual(dir);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("pack.json")).unwrap()).unwrap();
        manifest["counterfactual_definition"] = serde_json::json!("counterfactual.json");
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_run_inspect_pack_outputs_summary() {
        let dir = temp_pack_dir("inspect-pack");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "inspect-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Pack: Health Pack (health-pack)"));
        assert!(output.contains("Scenario: Pack Scenario (pack-scenario)"));
        assert!(output.contains("Experiment spec: yes"));
    }

    #[test]
    fn test_run_validate_pack_outputs_success() {
        let dir = temp_pack_dir("validate-pack");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "validate-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        assert_eq!(output, "Pack valid: Health Pack (health-pack)");
    }

    #[test]
    fn test_run_inspect_pack_shows_counterfactual_presence() {
        let dir = temp_pack_dir("inspect-pack-with-counterfactual");
        write_sample_pack_with_counterfactual(&dir);

        let output = run(&[
            "composure".into(),
            "inspect-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Counterfactual definition: yes"));
    }

    #[test]
    fn test_run_inspect_counterfactual_outputs_summary() {
        let dir = temp_pack_dir("inspect-counterfactual");
        let path = write_sample_counterfactual(&dir);

        let output = run(&[
            "composure".into(),
            "inspect-counterfactual".into(),
            path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Counterfactual: Recovery branch (cf-1)"));
        assert!(output.contains("Baseline: No change (baseline)"));
        assert!(output.contains("Candidate: Recovery (candidate)"));
    }

    #[test]
    fn test_run_validate_counterfactual_outputs_success() {
        let dir = temp_pack_dir("validate-counterfactual");
        let path = write_sample_counterfactual(&dir);

        let output = run(&[
            "composure".into(),
            "validate-counterfactual".into(),
            path.display().to_string(),
        ])
        .unwrap();

        assert_eq!(output, "Counterfactual valid: Recovery branch (cf-1)");
    }

    #[test]
    fn test_run_inspect_counterfactual_result_outputs_summary() {
        let dir = temp_pack_dir("inspect-counterfactual-result");
        let path = write_sample_counterfactual(&dir);
        let result_path = dir.join("counterfactual-result.json");

        let output = run(&[
            "composure".into(),
            "run-counterfactual".into(),
            path.display().to_string(),
            "--output".into(),
            result_path.display().to_string(),
        ])
        .unwrap();
        assert!(output.contains("Wrote artifact"));

        let inspect = run(&[
            "composure".into(),
            "inspect-counterfactual-result".into(),
            result_path.display().to_string(),
        ])
        .unwrap();

        assert!(inspect.contains("Counterfactual result:"));
        assert!(inspect.contains("Baseline summary:"));
        assert!(inspect.contains("Candidate summary:"));
    }

    #[test]
    fn test_run_inspect_pack_counterfactual_outputs_summary() {
        let dir = temp_pack_dir("inspect-pack-counterfactual");
        write_sample_pack_with_counterfactual(&dir);

        let output = run(&[
            "composure".into(),
            "inspect-pack-counterfactual".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Counterfactual: Recovery branch (cf-1)"));
        assert!(output.contains("Baseline: No change (baseline)"));
    }

    #[test]
    fn test_run_run_counterfactual_outputs_json() {
        let dir = temp_pack_dir("run-counterfactual");
        let path = write_sample_counterfactual(&dir);

        let output = run(&[
            "composure".into(),
            "run-counterfactual".into(),
            path.display().to_string(),
        ])
        .unwrap();

        let result: composure_core::CounterfactualResult = serde_json::from_str(&output).unwrap();
        assert_eq!(result.baseline.branch_id, "baseline");
        assert_eq!(result.candidate.branch_id, "candidate");
        assert!(result.comparison.metrics.end_delta > 0.0);
    }

    #[test]
    fn test_run_run_counterfactual_writes_output_file() {
        let dir = temp_pack_dir("run-counterfactual-output");
        let path = write_sample_counterfactual(&dir);
        let output_path = dir.join("counterfactual-result.json");

        let output = run(&[
            "composure".into(),
            "run-counterfactual".into(),
            path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let result: composure_core::CounterfactualResult = serde_json::from_str(&written).unwrap();
        assert_eq!(result.baseline.branch_id, "baseline");
        assert_eq!(result.candidate.branch_id, "candidate");
    }

    #[test]
    fn test_run_run_pack_counterfactual_outputs_json() {
        let dir = temp_pack_dir("run-pack-counterfactual");
        write_sample_pack_with_counterfactual(&dir);

        let output = run(&[
            "composure".into(),
            "run-pack-counterfactual".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        let result: composure_core::CounterfactualResult = serde_json::from_str(&output).unwrap();
        assert_eq!(result.baseline.branch_id, "baseline");
        assert_eq!(result.candidate.branch_id, "candidate");
        assert!(result.comparison.metrics.end_delta > 0.0);
    }

    #[test]
    fn test_run_run_pack_counterfactual_writes_output_file() {
        let dir = temp_pack_dir("run-pack-counterfactual-output");
        write_sample_pack_with_counterfactual(&dir);
        let output_path = dir.join("pack-counterfactual-result.json");

        let output = run(&[
            "composure".into(),
            "run-pack-counterfactual".into(),
            dir.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let result: composure_core::CounterfactualResult = serde_json::from_str(&written).unwrap();
        assert_eq!(result.baseline.branch_id, "baseline");
        assert_eq!(result.candidate.branch_id, "candidate");
    }

    #[test]
    fn test_run_run_pack_counterfactual_requires_counterfactual_definition() {
        let dir = temp_pack_dir("run-pack-counterfactual-missing");
        write_sample_pack(&dir);

        let err = run(&[
            "composure".into(),
            "run-pack-counterfactual".into(),
            dir.display().to_string(),
        ])
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("pack does not define a counterfactual_definition"));
    }

    #[test]
    fn test_run_run_pack_outputs_bundle_json() {
        let dir = temp_pack_dir("run-pack");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        let bundle: ExperimentBundle = serde_json::from_str(&output).unwrap();
        assert_eq!(bundle.spec.id, "pack-exp");
        assert_eq!(bundle.runs.len(), 1);
        assert_eq!(bundle.runs[0].run_id, "health-pack-run-1");
    }

    #[test]
    fn test_run_run_pack_accepts_manifest_path() {
        let dir = temp_pack_dir("run-pack-manifest");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.join("pack.json").display().to_string(),
        ])
        .unwrap();

        let bundle: ExperimentBundle = serde_json::from_str(&output).unwrap();
        assert_eq!(bundle.runs.len(), 1);
    }

    #[test]
    fn test_run_run_pack_writes_output_file() {
        let dir = temp_pack_dir("run-pack-output");
        write_sample_pack(&dir);
        let output_path = dir.join("bundle-output.json");

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let bundle: ExperimentBundle = serde_json::from_str(&written).unwrap();
        assert_eq!(bundle.runs.len(), 1);
    }

    #[test]
    fn test_run_run_pack_ignores_invalid_observed_and_sweep_inputs() {
        let dir = temp_pack_dir("run-pack-runtime-only");
        write_sample_pack(&dir);
        fs::write(dir.join("observed-trajectory.json"), "{not-json").unwrap();
        fs::write(dir.join("sweep-definition.json"), "{not-json").unwrap();

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        let bundle: ExperimentBundle = serde_json::from_str(&output).unwrap();
        assert_eq!(bundle.runs.len(), 1);
    }

    #[test]
    fn test_run_run_pack_requires_runtime_model() {
        let dir = temp_pack_dir("run-pack-missing-runtime");
        write_sample_pack(&dir);

        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("pack.json")).unwrap()).unwrap();
        manifest
            .as_object_mut()
            .unwrap()
            .remove("runtime_model")
            .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let err = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("pack does not define a runtime_model"));
    }

    #[test]
    fn test_run_export_bundle_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-bundle-markdown-input.json");
        let output_path = temp_dir.join("composure-cli-bundle-markdown-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_bundle()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-bundle-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Experiment Bundle: Baseline (exp-1)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_sweep_summary_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-sweep-summary-md-input.json");
        let output_path = temp_dir.join("composure-cli-sweep-summary-md-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_sweep_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-sweep-summary-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Sweep Summary: Sweep (sweep-1)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_sweep_csv_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-sweep-csv-input.json");
        let output_path = temp_dir.join("composure-cli-sweep-csv-output.csv");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_sweep_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-sweep-samples".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("case_id,is_best,objective,run_id,parameter_set_id,dose"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_sweep_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-sweep-md-input.json");
        let output_path = temp_dir.join("composure-cli-sweep-md-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_sweep_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-sweep-samples-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Sweep Samples: Sweep (sweep-1)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_calibration_csv_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-calibration-csv-input.json");
        let output_path = temp_dir.join("composure-cli-calibration-csv-output.csv");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_calibration_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-calibration-candidates".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("rank,is_best,case_id,parameter_set_id,run_id,score"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_calibration_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-calibration-md-input.json");
        let output_path = temp_dir.join("composure-cli-calibration-md-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_calibration_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-calibration-candidates-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Calibration Candidates: Dose Sweep (dose-sweep)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_summarize_monte_carlo_outputs_summary_json() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-summary-monte-carlo.json");

        fs::write(
            &path,
            serde_json::to_string(&sample_monte_carlo(1, [0.84, 0.74, 0.64])).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-monte-carlo".into(),
            path.display().to_string(),
        ])
        .unwrap();

        let summary: RunSummary = serde_json::from_str(&output).unwrap();
        assert_eq!(summary.monte_carlo.as_ref().unwrap().time_steps, 3);
        assert!(summary.composure.is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_run_summarize_monte_carlo_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-summary-input.json");
        let output_path = temp_dir.join("composure-cli-summary-output.json");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_monte_carlo(1, [0.84, 0.74, 0.64])).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-monte-carlo".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let summary: RunSummary = serde_json::from_str(&written).unwrap();
        assert_eq!(summary.monte_carlo.as_ref().unwrap().end, Some(0.64));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_summarize_bundle_run_outputs_summary_json() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-bundle.json");
        let bundle = sample_bundle();

        fs::write(&path, serde_json::to_string(&bundle).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-bundle-run".into(),
            path.display().to_string(),
            "run-1".into(),
        ])
        .unwrap();

        let summary: RunSummary = serde_json::from_str(&output).unwrap();
        assert!(summary.monte_carlo.is_none());
        assert!(summary.composure.is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_run_summarize_bundle_run_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let bundle_path = temp_dir.join("composure-cli-bundle-output.json");
        let output_path = temp_dir.join("composure-cli-bundle-summary-output.json");
        let bundle = sample_bundle();

        fs::write(&bundle_path, serde_json::to_string(&bundle).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-bundle-run".into(),
            bundle_path.display().to_string(),
            "run-1".into(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let summary: RunSummary = serde_json::from_str(&written).unwrap();
        assert!(summary.monte_carlo.is_none());

        let _ = fs::remove_file(bundle_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_inspect_report_outputs_summary() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-report-inspect.json");

        fs::write(&path, serde_json::to_string(&sample_report()).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "inspect-report".into(),
            path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Archetype change:"));
        assert!(output.contains("Comparison: rmse="));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_run_inspect_calibration_outputs_summary() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-calibration-inspect.json");

        fs::write(
            &path,
            serde_json::to_string(&sample_calibration_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "inspect-calibration".into(),
            path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Calibration: Dose Sweep (dose-sweep)"));
        assert!(output.contains("Best case: Some(\"dose-sweep-1\")"));

        let _ = fs::remove_file(path);
    }
}
