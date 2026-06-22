import { useState, useEffect } from 'react';
import * as api from '../lib/commands';

interface Card {
  id: string;
  front: string;
  back: string;
  page_path: string;
  ease_factor: number;
  interval_days: number;
  repetitions: number;
  next_review: string;
}

export default function FlashcardsPanel() {
  const [cards, setCards] = useState<Card[]>([]);
  const [current, setCurrent] = useState(0);
  const [showBack, setShowBack] = useState(false);
  const [message, setMessage] = useState('');

  useEffect(() => {
    api.generateFlashcards().then(setCards).catch(console.error);
  }, []);

  const review = async (quality: number) => {
    if (current >= cards.length) return;
    try {
      await api.reviewCard(cards[current].id, quality);
      setMessage(quality >= 3 ? 'Correct!' : 'Review again soon');
      setTimeout(() => {
        setShowBack(false);
        setMessage('');
        setCurrent(c => c + 1);
      }, 800);
    } catch (e) {
      setMessage(`Error: ${e}`);
    }
  };

  if (cards.length === 0) {
    return (
      <div className="p-4 max-w-2xl mx-auto">
        <h2 className="text-lg font-semibold mb-3">Flashcards</h2>
        <p className="text-sm text-gray-400">
          No flashcards found. Create blocks with <code>question::</code> and
          <code> answer::</code> properties to generate cards.
        </p>
      </div>
    );
  }

  if (current >= cards.length) {
    return (
      <div className="p-4 max-w-2xl mx-auto text-center">
        <h2 className="text-lg font-semibold mb-3">Session Complete!</h2>
        <p className="text-sm text-gray-400 mb-4">
          Reviewed {cards.length} card{cards.length !== 1 ? 's' : ''}.
        </p>
        <button
          onClick={() => { setCurrent(0); setShowBack(false); }}
          className="px-4 py-2 bg-[var(--accent-500)] text-white rounded text-sm hover:bg-[var(--accent-600)]"
        >
          Start Again
        </button>
      </div>
    );
  }

  const card = cards[current];

  return (
    <div className="p-4 max-w-2xl mx-auto">
      <h2 className="text-lg font-semibold mb-1">Flashcards</h2>
      <p className="text-xs text-gray-400 mb-4">
        Card {current + 1} of {cards.length}
        {card.next_review && ` · Next: ${card.next_review}`}
      </p>

      {/* Card */}
      <div
        className="min-h-[200px] flex items-center justify-center p-8 rounded-lg border-2 border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 cursor-pointer mb-4"
        onClick={() => setShowBack(!showBack)}
      >
        <div className="text-center max-w-lg">
          <p className="text-sm text-gray-400 mb-2">
            {showBack ? 'Answer' : 'Question'}
          </p>
          <p className="text-lg font-medium">
            {showBack ? card.back : card.front}
          </p>
          <p className="text-xs text-gray-400 mt-4">
            {showBack ? 'Click to see question' : 'Click to reveal answer'}
          </p>
        </div>
      </div>

      {/* Source */}
      <p className="text-xs text-gray-400 mb-3 text-center">
        Source: {card.page_path} · Ease: {card.ease_factor.toFixed(1)} · Interval: {card.interval_days}d
      </p>

      {message && (
        <div className="text-center text-sm mb-3 font-medium text-[var(--accent-600)]">
          {message}
        </div>
      )}

      {/* SM-2 rating buttons */}
      {showBack && !message && (
        <div className="flex justify-center gap-2">
          {[
            { label: 'Blackout', q: 0, color: 'bg-red-500 hover:bg-red-600' },
            { label: 'Hard', q: 2, color: 'bg-orange-500 hover:bg-orange-600' },
            { label: 'Good', q: 3, color: 'bg-green-500 hover:bg-green-600' },
            { label: 'Easy', q: 5, color: 'bg-[var(--accent-500)] hover:bg-[var(--accent-600)]' },
          ].map(({ label, q, color }) => (
            <button
              key={q}
              onClick={() => review(q)}
              className={`px-3 py-1.5 text-white text-xs rounded ${color}`}
            >
              {label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
