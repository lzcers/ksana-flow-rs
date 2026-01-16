import { useWorkflow } from './hooks/useWorkflow';
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
    workflowStatus,
    onNodesChange,
    onEdgesChange,
    onNodeDragStop,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
    runWorkflow,
    pauseWorkflow,
    resumeWorkflow,
    stopWorkflow,
    saveWorkflow,
    loadWorkflow,
    deleteWorkflow,
    renameWorkflow,
    createNewWorkflow
  } = workflow;

  const selectedNode = state.nodes.find(n => n.id === state.selectedNodeId);

  return (
    <WorkflowProvider value={workflow}>
      <div className="flex h-screen w-screen overflow-hidden bg-zinc-950 font-sans text-zinc-100 relative">
        <ReactFlowProvider>
          <Canvas
            nodes={state.nodes}
            edges={state.edges}
            workflowStatus={workflowStatus}
            availableNodes={nodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onNodeDragStop={onNodeDragStop}
            onConnect={onConnect}
            onAddNode={addNode}
            onRun={runWorkflow}
            onPause={pauseWorkflow}
            onResume={resumeWorkflow}
            onStop={stopWorkflow}
            workflows={workflows}
            currentWorkflowId={currentWorkflowId}
            onLoadWorkflow={loadWorkflow}
            onSaveWorkflow={saveWorkflow}
            onDeleteWorkflow={deleteWorkflow}
            onRenameWorkflow={renameWorkflow}
            onCreateNew={createNewWorkflow}
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
