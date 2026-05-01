import React, { useState, useEffect } from 'react';

interface TypewriterProps {
  text: string;
  speed?: number;
  onComplete?: () => void;
}

const Typewriter: React.FC<TypewriterProps> = ({ text, speed = 30, onComplete }) => {
  const [displayedText, setDisplayedText] = useState('');
  const [currentIndex, setCurrentIndex] = useState(0);

  useEffect(() => {
    // Reset when text changes
    setDisplayedText('');
    setCurrentIndex(0);
  }, [text]);

  useEffect(() => {
    if (currentIndex < text.length) {
      const timer = setTimeout(() => {
        setDisplayedText((prev) => prev + text[currentIndex]);
        setCurrentIndex((prev) => prev + 1);
      }, speed);
      return () => clearTimeout(timer);
    } else if (currentIndex === text.length) {
      if (onComplete) onComplete();
    }
  }, [currentIndex, text, speed, onComplete]);

  // Click to skip
  const handleSkip = () => {
    if (currentIndex < text.length) {
      setDisplayedText(text);
      setCurrentIndex(text.length);
      if (onComplete) onComplete();
    }
  };

  return (
    <div className="cursor-pointer h-full whitespace-pre-wrap" onClick={handleSkip}>
      {displayedText}
      {currentIndex < text.length && <span className="animate-pulse">|</span>}
    </div>
  );
};

export default Typewriter;
