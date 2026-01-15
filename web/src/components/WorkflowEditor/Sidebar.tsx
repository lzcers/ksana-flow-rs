import React, { useState } from 'react';
import { FileText, Plus, Save as SaveIcon, Trash2, Edit2, X, Check } from 'lucide-react';
import { cn } from '../../utils/cn';
import type { NodeMetadata } from '../../api';
import { NODE_TYPES } from './nodeTypes';
import type { WorkflowStatus } from '../../hooks/useWorkflow';

interface SidebarProps {
  nodeTypes: NodeMetadata[];
  workflows: { id: number; name: string }[];
  currentWorkflowId: number | null;
  workflowStatus: WorkflowStatus;
  onAddNode: (type: string) => void;
  onLoadWorkflow: (id: number) => void;
  onSaveWorkflow: (name?: string) => void;
  onDeleteWorkflow: (id: number) => void;
  onRenameWorkflow: (id: number, newName: string) => void;
  onCreateNew: () => void;
}

const getIcon = (name: string) => {
  const nodeType = NODE_TYPES.find(i => i.type === name);
  return nodeType?.icon || FileText;
};

const getColorForCategory = (category: string) => {
  switch (category.toLowerCase()) {
    case 'source': return 'bg-blue-900/30 text-blue-400 border border-blue-800/50';
    case 'strategy': return 'bg-purple-900/30 text-purple-400 border border-purple-800/50';
    case 'sink': return 'bg-orange-900/30 text-orange-400 border border-orange-800/50';
    default: return 'bg-zinc-800 text-zinc-400 border border-zinc-700';
  }
};

export const Sidebar: React.FC<SidebarProps> = ({
  nodeTypes,
  workflows,
  currentWorkflowId,
  workflowStatus,
  onAddNode,
  onLoadWorkflow,
  onSaveWorkflow,
  onDeleteWorkflow,
  onRenameWorkflow,
  onCreateNew
}) => {
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [newWorkflowName, setNewWorkflowName] = useState('');
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingName, setEditingName] = useState('');

  const handleSave = () => {
    if (currentWorkflowId) {
      onSaveWorkflow();
    } else {
      setShowSaveDialog(true);
    }
  };

  const confirmSave = () => {
    onSaveWorkflow(newWorkflowName);
    setShowSaveDialog(false);
    setNewWorkflowName('');
  };

  const startEditing = (id: number, name: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingId(id);
    setEditingName(name);
  };

  const cancelEditing = (e?: React.MouseEvent) => {
    e?.stopPropagation();
    setEditingId(null);
    setEditingName('');
  };

  const saveEditing = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (editingId && editingName.trim()) {
      onRenameWorkflow(editingId, editingName.trim());
      setEditingId(null);
      setEditingName('');
    }
  };

  return (
    <aside className="w-64 border-r border-zinc-800 bg-zinc-900 p-6 z-10 flex flex-col h-full overflow-hidden">
      <div className="mb-6 flex-shrink-0">
        <h1 className="text-lg font-bold tracking-tight text-zinc-100 flex items-center gap-2 mb-6 flex justify-center">
          Ksana Flow Engine
        </h1>

        <div className="space-y-2">
          <button
            onClick={onCreateNew}
            className="w-full flex items-center gap-2 px-3 py-2 text-sm font-medium text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 rounded-md transition-colors border border-zinc-800 hover:border-zinc-700"
          >
            <Plus size={16} />
            New Workflow
          </button>
          <div className="relative group">
            <button
              onClick={handleSave}
              className="w-full flex items-center gap-2 px-3 py-2 text-sm font-medium text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 rounded-md transition-colors border border-zinc-800 hover:border-zinc-700"
            >
              <SaveIcon size={16} />
              {currentWorkflowId ? 'Save Workflow' : 'Save As...'}
            </button>
          </div>
        </div>

        {showSaveDialog && (
          <div className="mt-2 p-3 bg-zinc-800 rounded-lg border border-zinc-700 shadow-xl">
            <input
              type="text"
              value={newWorkflowName}
              onChange={(e) => setNewWorkflowName(e.target.value)}
              placeholder="Workflow Name"
              className="w-full px-2 py-1 text-sm bg-zinc-900 border border-zinc-700 text-zinc-100 rounded mb-2 focus:outline-none focus:border-blue-500"
              autoFocus
            />
            <div className="flex gap-2 justify-end">
              <button onClick={() => setShowSaveDialog(false)} className="text-xs text-zinc-500 hover:text-zinc-300">Cancel</button>
              <button onClick={confirmSave} className="text-xs bg-blue-600 text-white px-2 py-1 rounded hover:bg-blue-500">Save</button>
            </div>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto min-h-0 space-y-6 pr-2">
        <div>
          <h2 className="text-[11px] font-bold text-zinc-500 uppercase tracking-[0.1em] mb-2 sticky top-0 bg-zinc-900 py-1">Workflows</h2>
          <div className="space-y-1">
            {workflows.map(wf => (
              <div
                key={wf.id}
                className={cn(
                  "group relative w-full flex items-center gap-2 px-3 py-2 text-sm rounded-md transition-colors cursor-pointer",
                  currentWorkflowId === wf.id
                    ? "bg-blue-900/20 text-blue-400 font-medium border border-blue-900/50"
                    : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 border border-transparent"
                )}
                onClick={() => {
                  if (editingId !== wf.id) onLoadWorkflow(wf.id);
                }}
              >
                <FileText size={14} className={editingId === wf.id ? "text-zinc-600" : ""} />

                {editingId === wf.id ? (
                  <div className="flex-1 flex items-center gap-1 min-w-0" onClick={e => e.stopPropagation()}>
                    <input
                      type="text"
                      value={editingName}
                      onChange={e => setEditingName(e.target.value)}
                      className="flex-1 min-w-0 px-1 py-0.5 text-xs border border-blue-500/50 rounded focus:outline-none focus:border-blue-500 bg-zinc-950 text-zinc-100"
                      autoFocus
                      onKeyDown={e => {
                        if (e.key === 'Enter') saveEditing(e as any);
                        if (e.key === 'Escape') cancelEditing(e as any);
                      }}
                    />
                    <button onClick={saveEditing} className="p-0.5 text-green-500 hover:bg-green-900/30 rounded"><Check size={12} /></button>
                    <button onClick={cancelEditing} className="p-0.5 text-red-500 hover:bg-red-900/30 rounded"><X size={12} /></button>
                  </div>
                ) : (
                  <>
                    <span className="truncate flex-1" onDoubleClick={(e) => startEditing(wf.id, wf.name, e)}>
                      {wf.name}
                    </span>
                    {currentWorkflowId === wf.id && workflowStatus !== 'idle' && (
                        <div className={cn(
                            "w-2 h-2 rounded-full",
                            workflowStatus === 'running' ? "bg-green-500 animate-pulse" : "bg-yellow-500"
                        )} title={workflowStatus} />
                    )}
                    <div className="flex items-center opacity-0 group-hover:opacity-100 transition-opacity gap-1">
                      <button
                        onClick={(e) => startEditing(wf.id, wf.name, e)}
                        className="p-1 hover:bg-zinc-700 text-zinc-500 hover:text-zinc-300 rounded transition-all"
                        title="Rename Workflow"
                      >
                        <Edit2 size={12} />
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (confirm('Are you sure you want to delete this workflow?')) {
                            onDeleteWorkflow(wf.id);
                          }
                        }}
                        className="p-1 hover:bg-red-900/30 hover:text-red-400 text-zinc-500 rounded transition-all"
                        title="Delete Workflow"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </>
                )}
              </div>
            ))}
            {workflows.length === 0 && (
              <div className="text-xs text-zinc-600 italic px-3 py-2">No saved workflows</div>
            )}
          </div>
        </div>

        <div>
          <h2 className="text-[11px] font-bold text-zinc-500 uppercase tracking-[0.1em] mb-2 sticky top-0 bg-zinc-900 py-1">Components</h2>
          <div className="space-y-1.5">
            {nodeTypes.map(nodeType => {
              const Icon = getIcon(nodeType.name);
              const colorClass = getColorForCategory(nodeType.category);
              return (
                <button
                  key={nodeType.name}
                  onClick={() => onAddNode(nodeType.name)}
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData('application/reactflow', nodeType.name);
                    e.dataTransfer.effectAllowed = 'move';
                  }}
                  className="w-full flex items-center gap-3 p-2 rounded-lg hover:bg-zinc-800 transition-colors text-left group cursor-grab active:cursor-grabbing border border-transparent hover:border-zinc-700"
                >
                  <div className={cn("p-1.5 rounded-md transition-colors", colorClass)}>
                    <Icon size={16} />
                  </div>
                  <div className="flex flex-col">
                    <span className="text-sm font-medium text-zinc-400 group-hover:text-zinc-100">
                      {nodeType.name}
                    </span>
                    <span className="text-[10px] text-zinc-600">{nodeType.category}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </aside>
  );
};
