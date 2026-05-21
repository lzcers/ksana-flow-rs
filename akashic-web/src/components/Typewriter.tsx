import React, { useEffect, useRef } from 'react';
import {
  Incremark,
  darkTheme,
  ThemeProvider,
  useIncremark,
} from '@incremark/react';
import '@incremark/theme/styles.css';

interface TypewriterProps {
  text: string;
  animate?: boolean;
  isFinished?: boolean;
  onComplete?: () => void;
}

const Typewriter: React.FC<TypewriterProps> = ({
  text,
  animate = true,
  isFinished = false,
  onComplete,
}) => {
  const hasCompletedRef = useRef(false);
  const onCompleteRef = useRef(onComplete);
  const previousTextRef = useRef('');
  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
    typewriter: {
      enabled: animate,
      effect: 'typing',
    },
  });

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
    incremark.typewriter.setEnabled(animate);
  }, [animate, incremark]);

  useEffect(() => {
    const previousText = previousTextRef.current;

    if (!text) {
      previousTextRef.current = '';
      incremark.reset();
      completeDisplay();
      return;
    }

    if (!animate) {
      previousTextRef.current = text;
      incremark.render(text);
      completeDisplay();
      return;
    }

    hasCompletedRef.current = false;

    if (!text.startsWith(previousText)) {
      incremark.reset();
      incremark.append(text);
    } else if (text.length > previousText.length) {
      incremark.append(text.slice(previousText.length));
    }

    if (isFinished) {
      incremark.finalize();
    }

    previousTextRef.current = text;
  }, [animate, incremark, isFinished, text]);

  useEffect(() => {
    if (!text || !animate || !isFinished || !incremark.isDisplayComplete) {
      return;
    }

    completeDisplay();
  }, [animate, incremark.isDisplayComplete, isFinished, text]);

  return (
    <div className="h-full">
      <ThemeProvider theme={darkTheme}>
        <Incremark incremark={incremark} />
      </ThemeProvider>
    </div>
  );
};

const MemoizedTypewriter = React.memo(Typewriter);

MemoizedTypewriter.displayName = 'Typewriter';

export default MemoizedTypewriter;
