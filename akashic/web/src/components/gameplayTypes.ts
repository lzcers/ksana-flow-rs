export interface NarrationRoundEntry {
  round: number;
  narrationText: string;
  narrationStatus: 'pending' | 'running' | 'done' | 'error' | null;
  selectedChoiceText: string | null;
  isAwaitingNarration: boolean;
}
