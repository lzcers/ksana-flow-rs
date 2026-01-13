import React from 'react';
import { useWorkflow } from './hooks/useWorkflow';
import { Sidebar } from './components/WorkflowEditor/Sidebar';
import { Canvas } from './components/WorkflowEditor/Canvas';
import { PropertyPanel } from './components/WorkflowEditor/PropertyPanel';
import { ReactFlowProvider } from '@xyflow/react';
import { ToastProvider } from './contexts/ToastContext';
import { WorkflowProvider } from './contexts/WorkflowContext';

export default function App() {
  return (
    <ToastProvider>
      <AppContent />
    </ToastProvider>
  );
}

function AppContent() {
  const workflow = useWorkflow();
  const {
    state,
    nodeTypes,
    workflows,
    currentWorkflowId,
    isRunning,
    onNodesChange,
    onEdgesChange,
    onNodeDragStop,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
    runWorkflow,
    saveWorkflow,
    loadWorkflow,
    deleteWorkflow,
    renameWorkflow,
    createNewWorkflow
  } = workflow;

  const selectedNode = state.nodes.find(n => n.id === state.selectedNodeId);

  return (
    <WorkflowProvider value={workflow}>
      <div className="flex h-screen w-screen overflow-hidden bg-white font-sans text-slate-800">
        <Sidebar
          nodeTypes={nodeTypes}
          workflows={workflows}
          currentWorkflowId={currentWorkflowId}
          isRunning={isRunning}
          onAddNode={addNode}
          onRun={runWorkflow}
          onLoadWorkflow={loadWorkflow}
          onSaveWorkflow={saveWorkflow}
          onDeleteWorkflow={deleteWorkflow}
          onRenameWorkflow={renameWorkflow}
          onCreateNew={createNewWorkflow}
        />

        <ReactFlowProvider>
          <Canvas
            nodes={state.nodes}
            edges={state.edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onNodeDragStop={onNodeDragStop}
            onConnect={onConnect}
            onAddNode={addNode}
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
    </WorkflowProvider>
  );
}
