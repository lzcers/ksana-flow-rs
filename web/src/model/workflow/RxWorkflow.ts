/**
 * RxWorkflow (Reactive Layer)
 * 包装 Core WorkflowModel，提供 RxJS 响应式接口。
 */

import { BehaviorSubject, Subject, Observable } from 'rxjs';
import { map, distinctUntilChanged, shareReplay } from 'rxjs/operators';
import type { WorkflowState, Node, Edge } from './types';
import type { GraphCommand } from './commands';
import { WorkflowModel } from './workflowModel';
import { applyCollapsedSubgraphUi } from './utils';
import type { Immutable } from 'immer';

export interface RxWorkflowOptions {
  initialState?: WorkflowState;
  enableLogging?: boolean;
}

export class RxWorkflow {
  private _model: WorkflowModel;

  // Subjects
  private _state$ = new BehaviorSubject<Immutable<WorkflowState>>({ nodes: [], edges: [] });
  private _commands$ = new Subject<GraphCommand>();
  private _history$ = new BehaviorSubject<{ canUndo: boolean; canRedo: boolean }>({ canUndo: false, canRedo: false });

  // Public Observables
  public readonly state$: Observable<Immutable<WorkflowState>>;
  public readonly viewState$: Observable<Immutable<WorkflowState>>; // 处理过 UI 逻辑（如折叠）的状态
  public readonly commands$ = this._commands$.asObservable();
  public readonly canUndo$ = this._history$.pipe(map(h => h.canUndo), distinctUntilChanged());
  public readonly canRedo$ = this._history$.pipe(map(h => h.canRedo), distinctUntilChanged());

  // Derived Observables (Helper)
  public readonly nodes$: Observable<Immutable<Node[]>>;
  public readonly edges$: Observable<Immutable<Edge[]>>;

  constructor(options: RxWorkflowOptions = {}) {
    this._model = new WorkflowModel({
      initialState: options.initialState,
      onStateChange: (newState) => {
        this._state$.next(newState);
        this._updateHistoryState();
      },
      onError: (error, command) => {
        console.error('[RxWorkflow] Error:', error, command);
      }
    });

    // 初始化 State Subject
    this._state$.next(this._model.state);
    this._updateHistoryState();

    // 基础 State 流
    this._state$ = new BehaviorSubject(this._model.state);
    this.state$ = this._state$.asObservable();

    // View State 流 (应用 UI 逻辑)
    this.viewState$ = this.state$.pipe(
      map(state => {
        const hasCollapsed = state.nodes.some(
          (n) =>
            (n.type === 'SubgraphNode' || n.type === 'MapNode') &&
            n.data.expanded === false
        );
        if (!hasCollapsed) return state;
        const { nodes, edges } = applyCollapsedSubgraphUi(state.nodes, state.edges);
        return { ...state, nodes, edges };
      }),
      shareReplay({ bufferSize: 1, refCount: true })
    );

    // 派生流
    this.nodes$ = this.state$.pipe(map(s => s.nodes), distinctUntilChanged());
    this.edges$ = this.state$.pipe(map(s => s.edges), distinctUntilChanged());
  }

  // ===== Public API =====

  /**
   * 分发 Command
   */
  dispatch(command: GraphCommand): void {
    // 异步广播 Command
    this._commands$.next(command);

    // 同步执行 Core Logic
    // 注意：目前的 Processor 都是同步的。如果有异步需求，应在外部处理完 Promise 后再 dispatch 结果 Command
    this._model.execute(command);
  }

  undo(): void {
    this.dispatch({ type: 'UNDO', payload: {} });
  }

  redo(): void {
    this.dispatch({ type: 'REDO', payload: {} });
  }


  // ===== Helper Methods (for compatibility) =====

  get currentState(): Immutable<WorkflowState> {
    return this._model.state;
  }

  getNodeData(nodeId: string): Immutable<Node['data']> | undefined {
    return this._model.state.nodes.find(n => n.id === nodeId)?.data;
  }

  destroy(): void {
    this._state$.complete();
    this._commands$.complete();
    this._history$.complete();
  }

  // ===== Private =====

  private _updateHistoryState(): void {
    this._history$.next({
      canUndo: this._model.canUndo,
      canRedo: this._model.canRedo
    });
  }
}
