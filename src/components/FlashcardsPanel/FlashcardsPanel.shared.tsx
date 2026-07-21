import { useState, useEffect, useCallback } from 'react';
import * as api from '../../lib/commands';

export interface CardData {
  id: string;
  front: string;
  back: string;
  page_path: string;
  ease_factor: number;
  interval_days: number;
  repetitions: number;
  next_review: string;
}

export const RATINGS = [
  { label: 'Blackout', q: 0, color: 'error' as const },
  { label: 'Hard', q: 2, color: 'warning' as const },
  { label: 'Good', q: 3, color: 'success' as const },
  { label: 'Easy', q: 5, color: 'primary' as const },
];

export interface ReviewState {
  cards: CardData[];
  current: number;
  showBack: boolean;
  message: string;
}

export function useFlashcardReview() {
  const [cards, setCards] = useState<CardData[]>([]);
  const [current, setCurrent] = useState(0);
  const [showBack, setShowBack] = useState(false);
  const [message, setMessage] = useState('');

  useEffect(() => {
    api.generateFlashcards().then(setCards).catch(console.error);
  }, []);

  const review = useCallback(async (quality: number) => {
    if (current >= cards.length) return;
    try {
      await api.reviewCard(cards[current].id, quality, cards[current].page_path);
      setMessage(quality >= 3 ? 'Correct!' : 'Review again soon');
      setTimeout(() => {
        setShowBack(false);
        setMessage('');
        setCurrent(c => c + 1);
      }, 800);
    } catch (e) {
      setMessage(`Error: ${e}`);
    }
  }, [current, cards]);

  const reset = useCallback(() => {
    setCurrent(0);
    setShowBack(false);
    setMessage('');
  }, []);

  const card = current < cards.length ? cards[current] : null;

  return { cards, current, showBack, setShowBack, message, review, reset, card };
}
