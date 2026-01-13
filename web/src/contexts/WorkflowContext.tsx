import { createContext, useContext } from 'react';
import type { useWorkflow } from '../hooks/useWorkflow';

type WorkflowContextType = ReturnType<typeof useWorkflow>;

const WorkflowContext = createContext<WorkflowContextType | null>(null);

export const WorkflowProvider = WorkflowContext.Provider;

export function useWorkflowContext() {
  const context = useContext(WorkflowContext);
  if (!context) {
    throw new Error('useWorkflowContext must be used within a WorkflowProvider');
  }
  return context;
}
