import {
    Activity,
    Box,
    Database,
    FileText,
    GitMerge,
    Group,
    Image,
    Mail,
    Repeat2,
    Sigma,
    Sparkles,
    Timer,
    type LucideIcon,
} from "lucide-react";
import type { NodeType } from "../../model/workflow/types";
import type { NodeMetadata as RegistryNodeMetadata } from "../../model/nodeRegistry";
import { BUILTIN_NODE_MANIFESTS } from "../nodes/manifests";
import type { NodeComponent } from "../nodes/nodeManifest";

export type NodeRegistryItem = {
    type: NodeType;
    label: string;
    icon: LucideIcon;
    color: string;
    metadata: RegistryNodeMetadata;
    Component: NodeComponent;
};

const ICON_BY_NAME: Record<string, LucideIcon> = {
    activity: Activity,
    bot: Sparkles,
    clock: Timer,
    "file-text": FileText,
    file: FileText,
    image: Image,
    layers: Repeat2,
    "git-branch": Group,
    "git-merge": Sigma,
    "line-chart": Box,
    mail: Mail,
    merge: GitMerge,
    play: Database,
    split: FileText,
};

const COLOR_BY_CATEGORY: Record<string, string> = {
    input: "text-slate-500 bg-slate-50",
    transform: "text-cyan-500 bg-cyan-50",
    flow: "text-zinc-500 bg-zinc-50",
    ai: "text-purple-500 bg-purple-50",
    trigger: "text-indigo-500 bg-indigo-50",
    logic: "text-orange-500 bg-orange-50",
    output: "text-indigo-500 bg-indigo-50",
};

export const NODE_REGISTRY: NodeRegistryItem[] = BUILTIN_NODE_MANIFESTS.map(manifest => {
        const metadata = manifest.metadata;
        const type = metadata.type as NodeType;
        const icon = metadata.icon ? ICON_BY_NAME[metadata.icon] : undefined;

        if (!icon) {
            return null;
        }

        return {
            type,
            label: metadata.displayName,
            icon,
            color: manifest.color ?? COLOR_BY_CATEGORY[metadata.category] ?? "text-zinc-500 bg-zinc-50",
            metadata,
            Component: manifest.Component,
        } satisfies NodeRegistryItem;
    })
    .filter((item): item is NodeRegistryItem => item !== null);

export const NODE_TYPES = NODE_REGISTRY.map(({ type, label, icon, color }) => ({ type, label, icon, color }));

export const NODE_COMPONENTS: Partial<Record<NodeType, NodeComponent>> = Object.fromEntries(
    NODE_REGISTRY.map(({ type, Component }) => [type, Component]),
);

export const NODE_REGISTRY_BY_TYPE = Object.fromEntries(
    NODE_REGISTRY.map(item => [item.type, item]),
) as Partial<Record<NodeType, NodeRegistryItem>>;

export function getNodeRegistryItem(type: string): NodeRegistryItem | undefined {
    return NODE_REGISTRY_BY_TYPE[type as NodeType];
}
