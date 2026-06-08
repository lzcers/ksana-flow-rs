import { memo } from "react";
import { type NodeProps } from "@xyflow/react";
import type { NodeData } from "@/model/workflow/types";
import { useImgGenNodeController } from "./hooks";
import { ImgGenNodeView } from "./view";

export const ImgGenNode = memo((props: NodeProps & { data: NodeData }) => {
    const { id, data, selected, width, height } = props;
    const controller = useImgGenNodeController({
        id,
        data,
        selected,
        width: typeof width === "number" && width !== 0 ? width : 200,
        height: typeof height === "number" && height !== 0 ? height : 200,
    });

    return <ImgGenNodeView {...props} {...controller} />;
});

ImgGenNode.displayName = "ImgGenNode";
