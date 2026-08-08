interface ChangeMetadata {
    author: string;
    timestamp: number;
    flags?: string[];
}

interface FieldDelta {
    fieldName: string;
    oldValue?: string;
    newValue?: string;
}

interface NodeDiff {
    nodeId: string;
    path: string[];
    deltas: FieldDelta[];
}

interface DiffReport {
    reportId: string;
    nodes: NodeDiff[];
    metadata: ChangeMetadata;
}

interface DiffSummary {
    summaryId: string;
    totalNodes: number;
    totalDeltas: number;
    primaryAuthor: string;
    affectedPaths: string[];
    sampleDelta: FieldDelta;
}

export function processDiff(report: DiffReport): DiffSummary {
    let totalDeltas = 0;
    const affectedPaths: string[] = [];
    let firstDelta: FieldDelta = { fieldName: "none" };

    for (const node of report.nodes) {
        totalDeltas += node.deltas.length;
        affectedPaths.push(node.path.join("."));
        if (node.deltas.length > 0 && firstDelta.fieldName === "none") {
            firstDelta = node.deltas[0];
        }
    }

    return {
        summaryId: `summary-${report.reportId}`,
        totalNodes: report.nodes.length,
        totalDeltas,
        primaryAuthor: report.metadata.author,
        affectedPaths,
        sampleDelta: firstDelta
    };
}
