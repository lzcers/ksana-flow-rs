import { useState, useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useParams } from 'react-router-dom';
import { useWorkflow } from './hooks/useWorkflow';
import { useAppInit } from './hooks/useAppInit';
import { Canvas } from './components/WorkflowEditor/Canvas';
import { PropertyPanel } from './components/WorkflowEditor/PropertyPanel';
import { WorkflowHeader } from './components/WorkflowEditor/WorkflowHeader';
import { ReactFlowProvider } from '@xyflow/react';
import { ToastContainer } from './components/ui/ToastContainer';

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/space/:spaceId/*" element={<WorkspaceWrapper />} />
        <Route path="/" element={<Navigate to="/space/ksana" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

function WorkspaceWrapper() {
  const { spaceId } = useParams<{ spaceId: string }>();

  // Guard against undefined spaceId (though routing ensures it's there)
  const currentSpaceId = spaceId || 'ksana';

  useAppInit(currentSpaceId);

  return (
    <>
      <AppContent />
      <ToastContainer />
    </>
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
    workflowStatuses,
    onNodesChange,
    onEdgesChange,
    onNodeDragStop,
    onConnect,
    addNode,
    runWorkflow,
    pauseWorkflow,
    resumeWorkflow,
    stopWorkflow,
    saveWorkflow,
    loadWorkflow,
    deleteWorkflow,
    renameWorkflow,
    createNewWorkflow,
    importWorkflow,
    getWorkflowBlueprint
  } = workflow;

  const [openTabs, setOpenTabs] = useState<{ id: number | null; name: string }[]>([]);

  const handleExportWorkflow = () => {
    const blueprint = getWorkflowBlueprint();
    const currentWf = workflows.find(w => w.id === currentWorkflowId);
    const name = currentWf?.name || 'workflow';
    const blob = new Blob([JSON.stringify({
      ...blueprint,
      name // Include name in export
    }, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${name}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleImportWorkflow = async (file: File) => {
    const text = await file.text();
    try {
      const data = JSON.parse(text);
      if (!data.nodes || !data.edges) {
        throw new Error("Invalid workflow format");
      }

      importWorkflow(data);

      setOpenTabs(prev => {
        const nullTab = prev.find(t => t.id === null);
        const newName = data.name || 'Imported Workflow';

        if (nullTab) {
          return prev.map(t => t.id === null ? { ...t, name: newName } : t);
        }
        return [...prev, { id: null, name: newName }];
      });

    } catch (e) {
      console.error("Failed to import workflow", e);
      alert('Failed to import workflow: Invalid file format');
    }
  };

  useEffect(() => {
    if (openTabs.length === 0) {
      if (currentWorkflowId !== null) {
        const wf = workflows.find(w => w.id === currentWorkflowId);
        if (wf) {
          setOpenTabs([{ id: currentWorkflowId, name: wf.name }]);
        }
      } else {
        // Only add 'New Workflow' tab if we are sure we are in that state (which is default)
        // But createNewWorkflow sets currentWorkflowId to null.
        // Let's just default to one new workflow tab if nothing is there.
        setOpenTabs([{ id: null, name: 'New Workflow' }]);
      }
    }
  }, []); // Run once on mount

  // Sync tab names when workflows change (e.g. rename)
  useEffect(() => {
    setOpenTabs(prev => prev.map(t => {
      if (t.id === null) return t;
      const wf = workflows.find(w => w.id === t.id);
      return wf ? { ...t, name: wf.name } : t;
    }));
  }, [workflows]);

  // When currentWorkflowId changes (e.g. after save), update the null tab if it became a saved workflow
  useEffect(() => {
    if (currentWorkflowId !== null) {
      // If we have a null tab and now we have an ID, it means we likely just saved.
      // Or we switched.
      // We need to be careful not to replace null tab if we just switched to an existing one.
      // But if we switched, handleLoadWorkflow would have handled it.
      // So this is mainly for "Save" action transforming New -> Saved.
      setOpenTabs(prev => {
        const nullTab = prev.find(t => t.id === null);
        const existingTab = prev.find(t => t.id === currentWorkflowId);

        // If we have a null tab, and we are now on a saved ID that isn't in tabs...
        // It's highly likely the null tab just got saved.
        if (nullTab && !existingTab) {
          const wf = workflows.find(w => w.id === currentWorkflowId);
          return prev.map(t => t.id === null ? { id: currentWorkflowId, name: wf?.name || t.name } : t);
        }
        return prev;
      });
    }
  }, [currentWorkflowId, workflows]);

  const handleLoadWorkflow = (id: number) => {
    loadWorkflow(id);
    setOpenTabs(prev => {
      if (prev.find(t => t.id === id)) return prev;
      const wf = workflows.find(w => w.id === id);
      return [...prev, { id, name: wf?.name || 'Unknown' }];
    });
  };

  const handleCreateNew = () => {
    createNewWorkflow();
    setOpenTabs(prev => {
      if (prev.find(t => t.id === null)) return prev;
      return [...prev, { id: null, name: 'New Workflow' }];
    });
  };

  const handleCloseTab = (id: number | null) => {
    const newTabs = openTabs.filter(t => t.id !== id);
    setOpenTabs(newTabs);

    // If we closed the active tab
    if (currentWorkflowId === id) {
      if (newTabs.length > 0) {
        const last = newTabs[newTabs.length - 1];
        if (last.id === null) {
          createNewWorkflow();
        } else {
          loadWorkflow(last.id);
        }
      } else {
        createNewWorkflow();
        setOpenTabs([{ id: null, name: 'New Workflow' }]);
      }
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-zinc-950 font-sans text-zinc-100">
      {/* Top Menu Bar */}
      <div className="h-10 border-b border-zinc-800 bg-zinc-900 px-3 flex items-center shrink-0">
        <WorkflowHeader
          workflows={workflows}
          currentWorkflowId={currentWorkflowId}
          workflowStatuses={workflowStatuses}
          onLoadWorkflow={handleLoadWorkflow}
          onSaveWorkflow={saveWorkflow}
          onDeleteWorkflow={deleteWorkflow}
          onRenameWorkflow={renameWorkflow}
          onCreateNew={handleCreateNew}
          onExportWorkflow={handleExportWorkflow}
          onImportWorkflow={handleImportWorkflow}
          tabs={openTabs}
          onCloseTab={handleCloseTab}
        />
      </div>

      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden relative">
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
          />
        </ReactFlowProvider>
      </div>
    </div>
  );
}
