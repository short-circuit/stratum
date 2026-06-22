import { useState } from 'react';
import * as api from '../lib/commands';

interface Props {
  pagePath: string;
}

interface BlockProp {
  block_id: string;
  properties: [string, string][];
  marker?: string;
  priority?: string;
}

export default function PropertiesPanel(_props: Props) {
  const [blockId, setBlockId] = useState('');
  const [prop, setProp] = useState<BlockProp | null>(null);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');

  const loadProperties = async () => {
    if (!blockId.trim()) return;
    try {
      const p = await api.getBlockProperties(blockId);
      setProp(p);
    } catch (e) {
      setProp(null);
    }
  };

  const addProperty = async () => {
    if (!newKey.trim() || !blockId.trim()) return;
    await api.setBlockProperty(blockId, newKey, newValue);
    setNewKey('');
    setNewValue('');
    loadProperties();
  };

  return (
    <div className="border-t border-neutral-200 dark:border-neutral-700 p-3">
      <h3 className="text-xs font-semibold text-neutral-500 uppercase mb-2">
        Properties
      </h3>

      <div className="flex gap-1 mb-2">
        <input
          type="text"
          value={blockId}
          onChange={e => setBlockId(e.target.value)}
          placeholder="Block UUID"
          className="flex-1 text-xs px-2 py-1 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800"
        />
        <button
          onClick={loadProperties}
          className="text-xs px-2 py-1 bg-[var(--accent-500)] text-white rounded hover:bg-[var(--accent-600)]"
        >
          Load
        </button>
      </div>

      {prop && (
        <div className="space-y-1">
          {prop.marker && (
            <div className="text-xs flex justify-between">
              <span className="text-neutral-500">Marker</span>
              <span className="font-medium">{prop.marker}</span>
            </div>
          )}
          {prop.priority && (
            <div className="text-xs flex justify-between">
              <span className="text-neutral-500">Priority</span>
              <span className="font-medium">{prop.priority}</span>
            </div>
          )}
          {prop.properties.map(([k, v], i) => (
            <div key={i} className="text-xs flex justify-between group">
              <span className="text-neutral-500">{k}</span>
              <span className="font-medium">{v}</span>
            </div>
          ))}
          {prop.properties.length === 0 && !prop.marker && (
            <p className="text-xs text-neutral-400">No properties set.</p>
          )}

          {/* Add property */}
          <div className="flex gap-1 mt-2 pt-2 border-t border-neutral-200 dark:border-neutral-700">
            <input
              type="text"
              value={newKey}
              onChange={e => setNewKey(e.target.value)}
              placeholder="Key"
              className="w-1/3 text-xs px-1.5 py-0.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800"
            />
            <input
              type="text"
              value={newValue}
              onChange={e => setNewValue(e.target.value)}
              placeholder="Value"
              className="flex-1 text-xs px-1.5 py-0.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800"
            />
            <button
              onClick={addProperty}
              className="text-xs px-2 py-0.5 bg-green-500 text-white rounded hover:bg-green-600"
            >
              +
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
