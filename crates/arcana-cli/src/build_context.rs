use arcana_package::{
    BuildDisposition, BuildExecutionContext, BuildProgress, BuildStatus, BuildTarget,
};

pub(crate) fn build_execution_context_for_target(
    target: &BuildTarget,
    product: Option<String>,
) -> Result<BuildExecutionContext, String> {
    if product.is_some() && !matches!(target, BuildTarget::WindowsDll) {
        return Err(
            "`--product` is only supported for the `windows-dll` native product target".to_string(),
        );
    }
    match target {
        BuildTarget::InternalAot | BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            Ok(BuildExecutionContext::with_selected_product(product))
        }
        BuildTarget::Other(other) => Err(format!("unsupported build target `{other}`")),
    }
}

pub(crate) fn render_build_progress(progress: BuildProgress<'_>) -> String {
    let verb = match progress.status.disposition() {
        BuildDisposition::Built => "Building",
        BuildDisposition::CacheHit => "Fresh",
    };
    let storage_key = progress.status.build_key().storage_key();
    render_build_progress_line(
        progress.index,
        progress.total,
        verb,
        progress.status.member_label(),
        &storage_key,
    )
}

fn render_build_progress_line(
    index: usize,
    total: usize,
    verb: &str,
    member: &str,
    storage_key: &str,
) -> String {
    format!(
        "[{}/{}] {:<8} {} {}",
        index,
        total.max(1),
        verb,
        member,
        storage_key
    )
}

pub(crate) fn render_build_completion(
    statuses: &[BuildStatus],
    target: &BuildTarget,
    product: Option<&str>,
) -> String {
    let built = statuses
        .iter()
        .filter(|status| status.disposition() == BuildDisposition::Built)
        .count();
    let fresh = statuses
        .iter()
        .filter(|status| status.disposition() == BuildDisposition::CacheHit)
        .count();
    let target_label = match product {
        Some(product) => format!("{target}@{product}"),
        None => target.to_string(),
    };
    render_build_completion_line(&target_label, statuses.len(), built, fresh)
}

fn render_build_completion_line(
    target_label: &str,
    package_count: usize,
    built: usize,
    fresh: usize,
) -> String {
    match package_count {
        1 => format!("Finished {target_label} build (1 package: {built} built, {fresh} fresh)"),
        count => {
            format!(
                "Finished {target_label} build ({count} packages: {built} built, {fresh} fresh)"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{render_build_completion_line, render_build_progress_line};

    #[test]
    fn render_build_progress_formats_built_status() {
        let line = render_build_progress_line(1, 3, "Building", "core", "windows-dll@api");
        assert_eq!(line, "[1/3] Building core windows-dll@api");
    }

    #[test]
    fn render_build_progress_formats_cache_hit_status() {
        let line = render_build_progress_line(2, 3, "Fresh", "core", "windows-dll@api");
        assert_eq!(line, "[2/3] Fresh    core windows-dll@api");
    }

    #[test]
    fn render_build_completion_summarizes_counts() {
        let line = render_build_completion_line("windows-dll@api", 2, 1, 1);
        assert_eq!(
            line,
            "Finished windows-dll@api build (2 packages: 1 built, 1 fresh)"
        );
    }
}
