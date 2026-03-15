use arcana_hir::{HirResolvedWorkspace, HirWorkspaceSummary};
use arcana_package::{WorkspaceFingerprints, WorkspaceGraph, compute_workspace_fingerprints};

use crate::CheckedWorkspace;

pub(crate) fn compute_member_fingerprints_for_checked_workspace(
    graph: &WorkspaceGraph,
    checked: &CheckedWorkspace,
) -> Result<WorkspaceFingerprints, String> {
    compute_member_fingerprints_for_workspace(
        graph,
        &checked.workspace,
        &checked.resolved_workspace,
    )
}

pub(crate) fn compute_member_fingerprints_for_workspace(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
) -> Result<WorkspaceFingerprints, String> {
    compute_workspace_fingerprints(graph, workspace, resolved_workspace)
}
