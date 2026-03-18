# 节点系统设计与自定义开发指南

本文档面向 `web/src/components/nodes` 这一套前端节点系统，说明它和工作流模型、后端节点注册之间的关系，以及如何新增一个完整可运行的节点。

## 1. 先看整体分层

当前节点系统不是单点实现，而是前后端协作的三层结构：

1. 后端节点注册
   `server/src/registry.rs`
   负责声明节点类型名、后端默认配置、运行时节点构造器，并通过 `/api/nodes` 暴露给前端。

2. 前端节点元数据注册
   `web/src/components/nodes/*/metadata.ts`
   `web/src/model/nodeRegistry/*`
   负责声明端口、显示名、分类、图标、前端默认配置、默认尺寸，以及连接校验需要的端口信息。

3. 前端节点组件注册
   `web/src/components/nodes/*/manifest.ts`
   `web/src/components/WorkflowEditor/nodeRegistry.ts`
   负责把节点类型映射到 React 组件，最终让画布能够渲染节点。

可以把它理解成：

```text
后端 registry
  -> /api/nodes
  -> 前端 loadMetadata()
  -> 允许在菜单中创建哪些节点 + 后端默认 config

前端 metadata
  -> 端口/图标/分类/默认尺寸/默认 config
  -> 节点新增、连线校验、Handle 渲染

前端 manifest
  -> type -> React Component
  -> Canvas/WorkflowNode 最终渲染
```

## 2. 核心对象与职责

### 2.1 后端 `NodeMetadata`

定义在 `server/src/registry.rs`：

- `name`: 节点类型名，必须和前端 `metadata.type` 完全一致
- `config`: 后端默认配置
- `inputs` / `outputs`: 后端视角的输入输出类型描述

这份元数据主要用于：

- `/api/nodes` 返回可用节点列表
- 新建节点时给前端一个后端默认 `config`
- 工作流运行时用节点类型名实例化真正的运行时节点

注意：后端这份元数据并不包含前端画布需要的端口位置、图标、分类、显示名。

### 2.2 前端 `NodeMetadata`

定义在 `web/src/model/nodeRegistry/types.ts`，典型实现在 `web/src/components/nodes/*/metadata.ts`。

它描述的是前端节点协议：

- `type`
- `displayName`
- `category`
- `icon`
- `ports.inputs` / `ports.outputs`
- `defaultConfig`
- `defaultSize`

这份元数据用于：

- 节点默认标题与默认尺寸
- `NodeWrapper` 自动渲染 Handle
- 连线合法性校验
- 数据端口类型兼容性判断
- 节点菜单图标、颜色、分类展示

### 2.3 前端 `NodeManifest`

定义在 `web/src/components/nodes/nodeManifest.ts`：

```ts
export interface NodeManifest {
  metadata: NodeMetadata;
  Component: NodeComponent;
  color?: string;
}
```

它是“前端元数据 + 前端组件”的绑定对象。没有 `manifest`，就算有 `metadata` 和组件，画布也不会渲染。

## 3. 目录约定

每个节点目录基本遵循下面的约定：

```text
web/src/components/nodes/MyNode/
  index.tsx        节点入口，组织 hooks 与 view
  view.tsx         纯视图层
  hooks.ts         可选，状态与交互逻辑
  metadata.ts      前端节点元数据
  manifest.ts      注册到前端节点系统
  styles.ts        可选，样式常量
```

常见共享基类与能力：

- `shared/NodeWrapper`
  通用节点壳，负责标题栏、运行按钮、Resize、Handle 渲染、错误提示
- `shared/FormNodeView`
  适合标准表单型节点
- `shared/hooks/useNodeConfig`
  更新 `data.config`
- `shared/hooks/useNodeConfigField`
  管理输入草稿与提交时机
- `shared/hooks/useNodeConfigValueField`
  针对字符串、布尔、数字字符串等常见配置字段做适配

## 4. 前端注册链路

新增一个节点后，前端至少要经过这条链路：

```text
MyNode/metadata.ts
MyNode/manifest.ts
  -> web/src/components/nodes/manifests.ts
  -> web/src/components/nodes/metadata.ts
  -> web/src/model/nodeRegistry/builtinNodes.ts
  -> web/src/model/nodeRegistry/registry.ts
  -> web/src/components/WorkflowEditor/nodeRegistry.ts
  -> Canvas / WorkflowNode / NodeContextMenu
```

其中几个关键点：

- `web/src/components/nodes/manifests.ts`
  决定哪些节点组件被前端打包为内置节点

- `web/src/components/nodes/metadata.ts`
  决定哪些前端元数据会注册到 `model/nodeRegistry`

- `web/src/components/WorkflowEditor/nodeRegistry.ts`
  把 `manifest.metadata.icon` 和 `manifest.metadata.category` 转成菜单展示需要的图标和颜色

如果你新增了新的 `icon` 名称或者新的 `category`，还要同步修改：

- `ICON_BY_NAME`
- `COLOR_BY_CATEGORY`

否则节点仍可运行，但菜单图标或颜色会退化甚至拿不到图标。

## 5. 节点在前端的生命周期

### 5.1 应用启动

`web/src/hooks/useAppInit.ts` 会触发：

- `loadMetadata()`
- `initializeWebSocket()`

`loadMetadata()` 在 `web/src/store/createWorkflow.ts` 里会请求 `/api/nodes`，然后只保留那些“前端已注册 manifest”的节点：

```ts
const allowedTypes = new Set(NODE_TYPES.map(nt => nt.type));
const filteredTypes = types.filter(t => allowedTypes.has(t.name));
```

这意味着：

- 后端注册了，但前端没 manifest：不会出现在菜单里
- 前端有 manifest，但后端没注册：菜单也不会出现，因为 `/api/nodes` 不会返回它

也就是说，当前系统的“节点可见性”必须前后端同时注册。

当前有一个明确例外：

- `SubgraphNode`
  它有前端 manifest，但不走 `/api/nodes` 菜单暴露，而是通过 `groupNodes()` 这类前端动作创建

### 5.2 在画布中创建节点

创建入口在：

- 右键菜单 `NodeContextMenu`
- 拖拽到画布
- `createCanvas.ts` 的 `addNode()`
- `groupNodes()` 这类特殊动作创建容器节点（如 `SubgraphNode`）

`addNode()` 会：

1. 用节点类型生成一个 `type-n` 形式的 id
2. 从后端返回的 `nodeTypes` 中找到同名节点，拿到后端默认 `config`
3. 调用工作流模型的 `action.addNode()`

最终在 `processAddNode()` 里组装节点数据：

- 标题来自前端 `metadata.displayName`
- 尺寸来自前端 `metadata.defaultSize`
- 配置来自前端 `defaultConfig` 与后端 `/api/nodes.config` 的合并
- 合并顺序是前端默认值在前，后端默认值覆盖在后

所以这里的约束很明确：

- 前端 `defaultConfig` 主要服务 UI 默认态
- 后端 `config` 代表真实运行默认值
- 两边最好保持一致，否则新建节点时以后端为准

### 5.3 画布渲染

渲染链路是：

```text
Canvas
  -> WorkflowNode
  -> NODE_COMPONENTS[type]
  -> 某个具体节点组件
```

`WorkflowNode` 只做一件事：按 `type` 找到对应的 React 组件。

真正的节点壳一般由 `NodeWrapper` 提供，它会：

- 显示节点标题
- 允许双击改名
- 显示运行按钮
- 支持 Resize
- 根据注册表自动渲染输入/输出端口
- 显示错误信息

### 5.4 连线校验

连接校验在 `web/src/model/workflow/utils/connection.ts`。

当前系统有两类端口：

- `control`
  控制流，决定执行顺序
- `data`
  数据流，传递值

Handle ID 编码规则定义在 `web/src/model/nodeRegistry/types.ts`：

- 控制流固定为 `ctrl`
- 数据流为 `data:{portId}`

连线时会做这些事情：

1. 根据节点类型拿到端口定义
2. 规范化 `sourceHandle` / `targetHandle`
3. 校验控制流和数据流不能混连
4. 校验数据类型兼容
5. 校验目标端口是否允许多连接
6. 生成边的 `data.kind/sourcePort/targetPort/dataType`

这套设计的直接结果是：

- 新节点只要把 `metadata.ports` 定义对，绝大多数连线行为都会自动成立
- 新节点不要再走旧版 `sourceHandles` / `targetHandles` 兼容模式，新的节点应统一走 `ports`

### 5.5 保存与恢复

工作流序列化在：

- `web/src/model/workflow/adapters/blueprintAdapter.ts`

它会把节点和边转成后端 Blueprint：

- 节点保留 `type`
- `data` 中的大部分字段都会持久化
- `parentId` / `extent` / `hidden` 会被保留
- 边会保留 `sourceHandle` / `targetHandle`
- 边的 `data.kind` 等数据会带到后端

恢复时：

- 从 Blueprint 还原节点尺寸、父子关系
- 根据 handle 推断边类型
- 对折叠的 `SubgraphNode` / `MapNode` 做 UI 代理边处理

### 5.6 运行时状态回写

运行态事件由 WebSocket 推进，入口在：

- `web/src/model/workflowManager/instance.ts`

关键回写字段：

- `data.status`
- `data.errorMessage`
- `data.inputs`
- `data.outputs`
- `data.lastMessage`
- `data.isOutputStream`

这意味着节点 UI 不应该自己维护“真实输出值”，而是优先从这些字段读取运行结果。

## 6. 自定义节点系统里的几个关键设计点

### 6.1 节点定义被拆成了两份元数据

这是当前系统最重要的设计现实：

- 后端 `NodeMetadata`: 用于可执行性、运行默认值、API 暴露
- 前端 `NodeMetadata`: 用于 UI、端口、连线、菜单展示

优点：

- 前端可以表达比后端更丰富的端口位置和展示信息
- 后端可以独立决定如何实例化运行时节点

代价：

- 节点类型名必须手动保持一致
- 默认配置存在重复定义
- 输入输出类型可能在两边出现漂移

所以新增节点时，第一优先级不是写 UI，而是保证“类型名、默认配置、端口语义”前后端一致。

### 6.2 `NodeWrapper` 是节点系统真正的基础设施

对于大多数节点，不要直接从零画卡片，优先基于 `NodeWrapper` 或 `FormNodeView`。

它已经帮你解决了：

- 统一标题栏
- 选中态
- 运行态
- Resize
- Handle 布局
- 标签编辑
- 错误展示

只要节点没有特别强的特殊交互，直接复用它能避免很多不一致。

### 6.3 配置更新约定为 `data.config`

静态配置统一放在 `node.data.config`，不要随意散落到 `node.data` 顶层。

推荐使用：

- `useNodeConfig`
- `useStringNodeConfigField`
- `useBooleanNodeConfigField`
- `useNumericStringNodeConfigField`

这样可以统一处理：

- 配置合并
- 焦点期间的草稿值
- IME 输入法 composition
- `change` / `blur` / `manual` 三种提交方式

### 6.4 容器节点和普通节点不是一回事

`SubgraphNode` 和 `MapNode` 不是普通叶子节点，它们本质上是“子图容器”。

前端特征：

- 节点内可以包含子节点
- 可以展开/折叠
- 折叠时会隐藏内部节点并生成 UI 代理边

后端特征：

- 在 `server/src/state.rs` 里通过 `compile_graph_with_groups()` 编译
- `SubgraphNode` 被编译成运行时 `SubgraphNode`
- `MapNode` 被编译成运行时 `SubgraphMapNode`
- `server/src/registry.rs` 里 `MapNode` 的 creator 只是占位；它不是普通 leaf node

所以如果你要新增“容器型节点”，不能只照着普通节点模板写，还要同步处理后端 group factory。

## 7. 参考实现怎么选

可以直接把现有节点当模板：

- `TimerNode`
  最简单的表单型节点，适合做最小骨架参考

- `ReduceNode`
  适合参考“选择器 + 条件字段 + 输出预览”

- `LLMNode`
  适合参考“复杂交互 + 配置面板 + 流式输出”

- `TextFileNode`
  适合参考“接入 store action / 文件上传”

- `ImgGenNode`
  适合参考“动态尺寸 + 预览态 + 复杂本地状态”

- `SubgraphNode` / `MapNode`
  适合参考“容器节点 / 子图节点”

## 8. 新增一个普通节点的推荐步骤

下面以“新增一个普通叶子节点”为例。

### 8.1 后端先注册运行时节点

如果这个节点需要真正运行，先在后端补齐：

1. 在 `nodes` crate 实现节点逻辑
2. 在 `server/src/registry.rs` 注册：
   - `name`
   - `config`
   - `inputs`
   - `outputs`
   - `creator`
3. 确保 `/api/nodes` 能返回这个类型

如果后端不注册：

- 节点不会出现在菜单里
- 即使前端手工造出 Blueprint，运行时也会因为类型不存在而失败

这里说的是普通叶子节点。像 `SubgraphNode` / `MapNode` 这种容器节点，运行时是通过 group 编译逻辑特殊处理的，不完全遵循这条规则。

### 8.2 在前端创建节点目录

建议最小结构：

```text
web/src/components/nodes/MyNode/
  index.tsx
  view.tsx
  metadata.ts
  manifest.ts
```

### 8.3 定义前端元数据

`metadata.ts` 示例：

```ts
import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const myNodeMetadata: NodeMetadata = {
  type: 'MyNode',
  displayName: 'My Node',
  category: 'transform',
  icon: 'file-text',
  description: '示例节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'input', label: 'Input', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'output', label: 'Output', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    text: '',
  },
  defaultSize: { width: 280, height: 160 },
};
```

这里最容易出错的是：

- `type` 必须和后端 `name` 一致
- `ports` 要符合真实语义
- `defaultConfig` 要和后端 `config` 保持一致

### 8.4 写节点组件

如果是普通表单节点，优先直接用 `FormNodeView`：

```ts
import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { MyNodeView } from './view';

export const MyNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const textField = useStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'text',
  });

  return <MyNodeView {...props} text={textField.draft} onTextChange={textField.onChange} />;
});
```

`view.tsx`：

```ts
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function MyNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  text,
  onTextChange,
}: NodeProps & { data: NodeData } & { text: string; onTextChange: (next: string) => void }) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={280}
      minHeight={160}
      groups={[
        {
          fields: [
            {
              kind: 'input',
              label: 'Text',
              value: text,
              onChange: onTextChange,
            },
          ],
        },
      ]}
    />
  );
}
```

### 8.5 定义 manifest

`manifest.ts`：

```ts
import { defineNodeManifest } from '../nodeManifest';
import { MyNode } from './index';
import { myNodeMetadata } from './metadata';

export const myNodeManifest = defineNodeManifest({
  metadata: myNodeMetadata,
  Component: MyNode,
});
```

### 8.6 把节点接入前端注册表

至少修改这些文件：

- `web/src/components/nodes/index.ts`
- `web/src/components/nodes/manifests.ts`
- `web/src/components/nodes/metadata.ts`

如果用了新的图标名或分类，还要改：

- `web/src/components/WorkflowEditor/nodeRegistry.ts`

### 8.7 验证

至少检查这些点：

1. 页面刷新后右键菜单能看到新节点
2. 新建节点时标题、尺寸、默认配置正确
3. Handle 位置、数量、类型正确
4. 连线能按预期通过或被拒绝
5. 保存后刷新，节点配置和边能恢复
6. 运行时后端能实例化该节点
7. WebSocket 回来的状态和输出能在 UI 上显示

## 9. 新增容器型节点的额外步骤

如果你要新增的是类似 `SubgraphNode` / `MapNode` 的容器节点，还要额外处理：

1. 前端折叠/展开状态与尺寸持久化
2. 子节点 `parentId` / `extent="parent"` 管理
3. 折叠态下的代理边逻辑
4. 后端 `compile_graph_with_groups()` 的 group factory
5. 子图运行态同步到当前画布

换句话说，容器节点不是“多写几个样式”的问题，而是工作流编译模型的一部分。

## 10. 当前系统的几个约束与建议

### 10.1 建议统一维护前后端默认配置

当前前后端都保存一份默认配置，新增节点时建议先以后端为准，再把前端 `defaultConfig` 对齐。

### 10.2 新节点不要再使用旧 Handle 兼容模式

`NodeWrapper` 还保留了：

- `sourceHandles`
- `targetHandles`

这只是兼容旧节点的保底逻辑。新节点应统一依赖 `metadata.ports`。

### 10.3 运行态输出优先读 `lastMessage` 和 `outputs`

对于能运行的节点，UI 展示输出时优先读取：

- `data.lastMessage`
- `data.outputs`

不要只看 `config.output` 这类字段，因为它更多是编辑态缓存，不是统一运行态协议。

### 10.4 动态尺寸或动态端口需要主动刷新 internals

如果节点会因为展开/折叠、端口数量变化、尺寸变化而改变 Handle 布局，参考：

- `SubgraphNodeView`
- `MapNodeView`

使用 `useUpdateNodeInternals(id)` 主动刷新 xyflow 内部布局。

## 11. 一句话总结

当前节点系统的核心思想是：

- 后端负责“这个节点能不能运行”
- 前端 `metadata` 负责“这个节点怎么连、怎么显示”
- 前端 `manifest` 负责“这个节点渲染哪个 React 组件”
- `NodeWrapper` 负责把通用节点能力收敛成统一外壳

新增节点时，只改其中一层通常都不够；要把“后端注册、前端 metadata、前端 manifest”当成一个完整闭环来做。
