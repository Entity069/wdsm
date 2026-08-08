from dataclasses import dataclass
from typing import Optional


@dataclass
class ChangeMetadata:
    author: str
    timestamp: float
    labels: Optional[list[str]]


@dataclass
class FieldDelta:
    field_name: str
    old_value: Optional[str]
    new_value: Optional[str]


@dataclass
class NodeDiff:
    node_id: str
    path: list[str]
    deltas: list[FieldDelta]


@dataclass
class DiffReport:
    report_id: str
    nodes: list[NodeDiff]
    metadata: ChangeMetadata


@dataclass
class DiffSummary:
    summary_id: str
    total_nodes: float
    total_deltas: float
    primary_author: str
    affected_paths: list[str]
    sample_delta: FieldDelta


class WitWorld:
    def process_diff(self, report: DiffReport) -> DiffSummary:
        """Process a deeply nested DiffReport and return a DiffSummary."""
        total_deltas = 0.0
        affected_paths: list[str] = []
        sample = FieldDelta(field_name="none", old_value=None, new_value=None)

        for node in report.nodes:
            total_deltas += float(len(node.deltas))
            affected_paths.append(".".join(node.path))
            if len(node.deltas) > 0 and sample.field_name == "none":
                sample = node.deltas[0]

        return DiffSummary(
            summary_id=f"summary-{report.report_id}",
            total_nodes=float(len(report.nodes)),
            total_deltas=total_deltas,
            primary_author=report.metadata.author,
            affected_paths=affected_paths,
            sample_delta=sample,
        )
