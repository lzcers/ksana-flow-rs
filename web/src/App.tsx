import React from 'react';
import { useWorkflow } from './hooks/useWorkflow';
import { Sidebar } from './components/WorkflowEditor/Sidebar';
import { Canvas } from './components/WorkflowEditor/Canvas';
import { PropertyPanel } from './components/WorkflowEditor/PropertyPanel';
import { ReactFlowProvider } from '@xyflow/react';

export default function App() {
  const {
    state,
    onNodesChange,
    onEdgesChange,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
  } = useWorkflow();

  const selectedNode = state.nodes.find(n => n.id === state.selectedNodeId);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-white font-sans text-slate-800">
      <Sidebar onAddNode={addNode} />

      <ReactFlowProvider>
        <Canvas
          nodes={state.nodes}
          edges={state.edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
        />
      </ReactFlowProvider>

      {selectedNode && (
        <PropertyPanel
          node={selectedNode}
          onUpdateData={updateNodeData}
          onDelete={deleteNode}
        />
      )}
    </div>
  );
}
