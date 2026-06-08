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
  const [visibleLength, setVisibleLength] = useState(() => (animate && text ? 0 : text.length));
  const renderedLength = animate ? Math.min(visibleLength, text.length) : text.length;

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
    if (text && animate && !isFinished) {
      hasCompletedRef.current = false;
    }
  }, [animate, isFinished, text]);

  useEffect(() => {
    if (!text || !animate) {
      completeDisplay();
    }
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
    if (!text || !animate || !isFinished || renderedLength < text.length) {
      return;
    }

    completeDisplay();
  }, [animate, isFinished, renderedLength, text]);

  return (
    <p className="whitespace-pre-wrap break-words text-inherit">
      {text.slice(0, renderedLength)}
    </p>
  );
};

const MemoizedTypewriter = React.memo(Typewriter);

MemoizedTypewriter.displayName = 'Typewriter';

export default MemoizedTypewriter;
