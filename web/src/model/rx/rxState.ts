/**
 * RxState - RxJS 状态管理类
 * 使用 BehaviorSubject 管理状态，暴露 Observable 供订阅
 */

import { BehaviorSubject, Observable } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import type { WorkflowState, Node, Edge } from '../types';

export interface RxWorkflowStateOptions {
  initialState?: WorkflowState;
}

export class RxWorkflowState {
  private _state$: BehaviorSubject<WorkflowState>;

  // 公共 Observable
  public readonly state$: Observable<WorkflowState>;
  public readonly nodes$: Observable<Node[]>;
  public readonly edges$: Observable<Edge[]>;
  public readonly selectedNodeId$: Observable<string | null>;

  constructor(options: RxWorkflowStateOptions = {}) {
    const initialState: WorkflowState = options.initialState ?? {
      nodes: [],
      edges: [],
      selectedNodeId: null,
    };

    this._state$ = new BehaviorSubject<WorkflowState>(initialState);

    // 创建派生 Observable
    this.state$ = this._state$.asObservable();

    this.nodes$ = this.state$.pipe(
      map((state) => state.nodes),
      distinctUntilChanged((prev, curr) => {
        if (prev.length !== curr.length) return false;
        // 简化的深度比较，实际使用时可能需要更精确的比较
        return prev.every((node, index) => node.id === curr[index]?.id);
      })
    );

    this.edges$ = this.state$.pipe(
      map((state) => state.edges),
      distinctUntilChanged((prev, curr) => {
        if (prev.length !== curr.length) return false;
        return prev.every((edge, index) => edge.id === curr[index]?.id);
      })
    );

    this.selectedNodeId$ = this.state$.pipe(
      map((state) => state.selectedNodeId),
      distinctUntilChanged()
    );
  }

  // ===== 公共方法 =====

  /**
   * 获取当前状态快照
   */
  get state(): WorkflowState {
    return this._state$.getValue();
  }

  /**
   * 更新状态
   * @param updater 状态更新函数
   */
  setState(updater: (state: WorkflowState) => WorkflowState): void {
    const nextState = updater(this.state);
    this._state$.next(nextState);
  }

  /**
   * 直接设置新状态
   * @param newState 新状态
   */
  next(newState: WorkflowState): void {
    this._state$.next(newState);
  }

  /**
   * 创建针对特定节点的 Observable
   * @param nodeId 节点ID
   */
  selectNode$(nodeId: string): Observable<Node | undefined> {
    return this.nodes$.pipe(
      map((nodes) => nodes.find((n) => n.id === nodeId)),
      distinctUntilChanged((prev, curr) => {
        if (!prev && !curr) return true;
        if (!prev || !curr) return false;
        return prev.id === curr.id; // 简化比较
      })
    );
  }

  /**
   * 创建针对特定边的 Observable
   * @param edgeId 边ID
   */
  selectEdge$(edgeId: string): Observable<Edge | undefined> {
    return this.edges$.pipe(
      map((edges) => edges.find((e) => e.id === edgeId)),
      distinctUntilChanged()
    );
  }

  /**
   * 获取连接到指定节点的边
   * @param nodeId 节点ID
   */
  getConnectedEdges(nodeId: string): Edge[] {
    return this.state.edges.filter(
      (e) => e.source === nodeId || e.target === nodeId
    );
  }

  /**
   * 销毁（清理订阅）
   */
  destroy(): void {
    this._state$.complete();
  }
}
