import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Plus, Save, Edit2, Check, X, Trash2, FileText } from 'lucide-react';
import { cn } from '../../utils/cn';
import type { WorkflowStatus } from '../../hooks/useWorkflow';

interface WorkflowHeaderProps {
  workflows: { id: number; name: string }[];
  currentWorkflowId: number | null;
  workflowStatus: WorkflowStatus;
  onLoadWorkflow: (id: number) => void;
  onSaveWorkflow: (name?: string) => void;
  onDeleteWorkflow: (id: number) => void;
  onRenameWorkflow: (id: number, newName: string) => void;
  onCreateNew: () => void;
}

export const WorkflowHeader: React.FC<WorkflowHeaderProps> = ({
  workflows,
  currentWorkflowId,
  workflowStatus,
  onLoadWorkflow,
  onSaveWorkflow,
  onDeleteWorkflow,
  onRenameWorkflow,
  onCreateNew
}) => {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editingName, setEditingName] = useState('');
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [newWorkflowName, setNewWorkflowName] = useState('');
  const [unsavedName, setUnsavedName] = useState('New Workflow');
  const dropdownRef = useRef<HTMLDivElement>(null);

  const currentWorkflow = workflows.find(w => w.id === currentWorkflowId);
  
  // Use currentWorkflow name if available, otherwise use unsavedName
  const displayName = currentWorkflow ? currentWorkflow.name : unsavedName;

  useEffect(() => {
    // Reset unsaved name when switching to a new/empty workflow context
    if (currentWorkflowId === null) {
      setUnsavedName('New Workflow');
    }
  }, [currentWorkflowId]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsDropdownOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSave = () => {
    if (currentWorkflowId) {
      onSaveWorkflow();
    } else {
      // If we have a custom name set via rename, use it
      if (unsavedName !== 'New Workflow') {
        onSaveWorkflow(unsavedName);
      } else {
        setShowSaveDialog(true);
      }
    }
  };

  const confirmSave = () => {
    if (newWorkflowName.trim()) {
      onSaveWorkflow(newWorkflowName);
      setShowSaveDialog(false);
      setNewWorkflowName('');
    }
  };

  const startEditing = () => {
    setEditingName(displayName);
    setIsEditing(true);
  };

  const saveEditing = () => {
    const trimmedName = editingName.trim();
    if (trimmedName) {
      if (currentWorkflowId) {
        onRenameWorkflow(currentWorkflowId, trimmedName);
      } else {
        setUnsavedName(trimmedName);
      }
      setIsEditing(false);
    }
  };

  const cancelEditing = () => {
    setIsEditing(false);
    setEditingName('');
  };

  return (
    <div className="flex items-center gap-2 bg-zinc-900/90 backdrop-blur border border-zinc-800 p-1.5 rounded-lg shadow-xl pointer-events-auto">
       {/* Workflow Selector */}
       <div className="relative" ref={dropdownRef}>
        <button
          onClick={() => setIsDropdownOpen(!isDropdownOpen)}
          className="flex items-center gap-2 px-3 py-1.5 hover:bg-zinc-800 rounded-md transition-colors min-w-[200px] justify-between border border-transparent hover:border-zinc-700"
        >
            <div className="flex items-center gap-2 overflow-hidden">
                <FileText size={16} className="text-blue-500" />
                <span className="text-sm font-medium text-zinc-200 truncate max-w-[150px]">
                    {displayName}
                </span>
            </div>
            <ChevronDown size={14} className="text-zinc-500" />
        </button>

        {isDropdownOpen && (
            <div className="absolute top-full left-0 mt-1 w-64 bg-zinc-900 border border-zinc-800 rounded-lg shadow-xl overflow-hidden py-1 max-h-[300px] overflow-y-auto">
                <button
                    onClick={() => {
                        onCreateNew();
                        setIsDropdownOpen(false);
                    }}
                    className="w-full flex items-center gap-2 px-3 py-2 text-sm text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors border-b border-zinc-800/50"
                >
                    <Plus size={14} />
                    New Workflow
                </button>
                {workflows.map(wf => (
                    <div key={wf.id} className="group flex items-center justify-between px-3 py-2 hover:bg-zinc-800 cursor-pointer"
                        onClick={() => {
                            onLoadWorkflow(wf.id);
                            setIsDropdownOpen(false);
                        }}
                    >
                        <span className={cn("text-sm truncate", currentWorkflowId === wf.id ? "text-blue-400" : "text-zinc-300")}>
                            {wf.name}
                        </span>
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                if (confirm('Are you sure you want to delete this workflow?')) {
                                    onDeleteWorkflow(wf.id);
                                }
                            }}
                            className="opacity-0 group-hover:opacity-100 p-1 text-zinc-500 hover:text-red-400 transition-opacity"
                        >
                            <Trash2 size={12} />
                        </button>
                    </div>
                ))}
                {workflows.length === 0 && (
                    <div className="px-3 py-2 text-xs text-zinc-500 italic">No workflows found</div>
                )}
            </div>
        )}
       </div>

       <div className="w-px h-6 bg-zinc-800 mx-1"></div>

       {/* Actions */}
       {/* Always show actions, allowing rename/save for unsaved workflows too */}
       <>
           {isEditing ? (
               <div className="flex items-center gap-1">
                   <input
                       type="text"
                       value={editingName}
                       onChange={(e) => setEditingName(e.target.value)}
                       className="w-32 px-2 py-1 text-sm bg-zinc-950 border border-blue-500/50 rounded focus:outline-none focus:border-blue-500 text-zinc-100"
                       autoFocus
                       onKeyDown={(e) => {
                           if (e.key === 'Enter') saveEditing();
                           if (e.key === 'Escape') cancelEditing();
                       }}
                   />
                   <button onClick={saveEditing} className="p-1 text-green-500 hover:bg-green-900/30 rounded"><Check size={14} /></button>
                   <button onClick={cancelEditing} className="p-1 text-red-500 hover:bg-red-900/30 rounded"><X size={14} /></button>
               </div>
           ) : (
               <button
                   onClick={startEditing}
                   className="p-1.5 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 rounded transition-colors"
                   title="Rename"
               >
                   <Edit2 size={16} />
               </button>
           )}

           <button
               onClick={handleSave}
               className="p-1.5 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 rounded transition-colors"
               title="Save"
           >
               <Save size={16} />
           </button>
       </>

       {showSaveDialog && (
          <div className="absolute top-full left-0 mt-2 p-3 bg-zinc-800 rounded-lg border border-zinc-700 shadow-xl w-64 z-50">
            <h3 className="text-xs font-bold text-zinc-400 uppercase mb-2">Save New Workflow</h3>
            <input
              type="text"
              value={newWorkflowName}
              onChange={(e) => setNewWorkflowName(e.target.value)}
              placeholder="Workflow Name"
              className="w-full px-2 py-1 text-sm bg-zinc-900 border border-zinc-700 text-zinc-100 rounded mb-2 focus:outline-none focus:border-blue-500"
              autoFocus
              onKeyDown={(e) => {
                  if (e.key === 'Enter') confirmSave();
                  if (e.key === 'Escape') setShowSaveDialog(false);
              }}
            />
            <div className="flex gap-2 justify-end">
              <button onClick={() => setShowSaveDialog(false)} className="text-xs text-zinc-500 hover:text-zinc-300">Cancel</button>
              <button onClick={confirmSave} className="text-xs bg-blue-600 text-white px-2 py-1 rounded hover:bg-blue-500">Save</button>
            </div>
          </div>
        )}
    </div>
  );
};
