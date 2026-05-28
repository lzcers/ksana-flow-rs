export interface NarrationRoundEntry {
  round: number;
  title: string;
  narrationText: string;
  narrationStatus: 'pending' | 'running' | 'done' | 'error' | null;
  selectedChoiceText: string | null;
  isAwaitingNarration: boolean;
}
