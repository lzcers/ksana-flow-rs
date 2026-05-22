import React, { useEffect, useRef, useState } from 'react';

interface TypewriterProps {
  text: string;
  animate?: boolean;
  isFinished?: boolean;
  onComplete?: () => void;
}

const CHARS_PER_TICK = 2;
const TICK_MS = 18;

const Typewriter: React.FC<TypewriterProps> = ({
  text,
  animate = true,
  isFinished = false,
  onComplete,
}) => {
  const hasCompletedRef = useRef(false);
  const onCompleteRef = useRef(onComplete);
  const previousTextRef = useRef('');
  const [visibleLength, setVisibleLength] = useState(0);

  const completeDisplay = () => {
    if (!hasCompletedRef.current) {
      hasCompletedRef.current = true;
      onCompleteRef.current?.();
    }
  };

  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  useEffect(() => {
    if (!text) {
      previousTextRef.current = '';
      setVisibleLength(0);
      completeDisplay();
      return;
    }

    if (!animate) {
      previousTextRef.current = text;
      setVisibleLength(text.length);
      completeDisplay();
      return;
    }

    if (!text.startsWith(previousTextRef.current)) {
      setVisibleLength(0);
    }

    hasCompletedRef.current = false;
    previousTextRef.current = text;
  }, [animate, text]);

  useEffect(() => {
    if (!text || !animate || visibleLength >= text.length) {
      return;
    }

    const timer = window.setTimeout(() => {
      setVisibleLength((prev) => Math.min(prev + CHARS_PER_TICK, text.length));
    }, TICK_MS);

    return () => window.clearTimeout(timer);
  }, [animate, text, visibleLength]);

  useEffect(() => {
    if (!text || !animate || !isFinished || visibleLength < text.length) {
      return;
    }

    completeDisplay();
  }, [animate, isFinished, text, visibleLength]);

  return (
    <p className="whitespace-pre-wrap break-words text-inherit">
      {text.slice(0, visibleLength)}
    </p>
  );
};

const MemoizedTypewriter = React.memo(Typewriter);

MemoizedTypewriter.displayName = 'Typewriter';

export default MemoizedTypewriter;
