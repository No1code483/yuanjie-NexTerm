import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { HotTable } from '@handsontable/react';
import { registerAllModules } from 'handsontable/registry';
import 'handsontable/dist/handsontable.full.min.css';
type CellChange = [number, string | number | ((...args: any[]) => any), any, any];
registerAllModules();
interface SheetData {
  name: string;
  rows: string[][];
  row_count: number;
  col_count: number;
}
interface TableEditorProps {
  filePath: string;
  fileExt: string;
  initialSheets: SheetData[];
  fileName: string;
  onSave: () => void;
  onClose: () => void;
  showStatus: (type: 'success' | 'error', text: string) => void;
}
export default function TableEditor({
  filePath,
  fileExt,
  initialSheets,
  fileName,
  onSave,
  onClose,
  showStatus
}: TableEditorProps) {
  const [sheets, setSheets] = useState<SheetData[]>(() => initialSheets.map(s => ({
    ...s,
    rows: s.rows.length > 0 ? s.rows : [['']]
  })));
  const [activeSheet, setActiveSheet] = useState(0);
  const [isSaving, setIsSaving] = useState(false);
  const [unsaved, setUnsaved] = useState(false);
  const sheetsRef = useRef(sheets);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    sheetsRef.current = sheets;
  }, [sheets]);
  const currentSheet = sheets[activeSheet];
  const handleSave = useCallback(async () => {
    setIsSaving(true);
    try {
      const {
        ipc
      } = await import('@/lib/ipc');
      const res = await ipc.invoke<any>('tableedit_write', {
        path: filePath,
        ext: fileExt,
        sheets: sheetsRef.current.map(s => ({
          name: s.name,
          rows: s.rows,
          row_count: s.row_count,
          col_count: s.col_count
        }))
      });
      if (res.code === 0) {
        setUnsaved(false);
        showStatus('success', t("components.TableEditor.k1"));
        onSave();
      } else {
        showStatus('error', res.message || t("errors.saveFailed"));
      }
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.saveFailed"));
    } finally {
      setIsSaving(false);
    }
  }, [filePath, fileExt, onSave, showStatus]);
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleSave]);
  useEffect(() => {
    if (!unsaved) return;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(async () => {
      try {
        const {
          ipc
        } = await import('@/lib/ipc');
        await ipc.invoke<any>('tableedit_write', {
          path: filePath,
          ext: fileExt,
          sheets: sheetsRef.current.map(s => ({
            name: s.name,
            rows: s.rows,
            row_count: s.row_count,
            col_count: s.col_count
          }))
        });
      } catch {
        /* silent */
      }
    }, 5000);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, [sheets]);
  const handleClose = useCallback(() => {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    onClose();
  }, [onClose]);
  const handleChange = useCallback((_index: number, _amount: number, source?: string) => {
    if (source === 'loadData') return;
    setUnsaved(true);
  }, []);
  const handleAfterChange = useCallback((changes: CellChange[] | null, source: string) => {
    if (!changes || source === 'loadData') return;
    setSheets(prev => {
      const next = prev.map(s => ({
        ...s,
        rows: s.rows.map(r => [...r])
      }));
      const sheet = next[activeSheet];
      for (const [row, prop,, newVal] of changes) {
        if (typeof prop === 'function') continue;
        const col = typeof prop === 'number' ? prop : parseInt(String(prop), 10);
        if (!isNaN(col)) {
          while (sheet.rows.length <= row) {
            sheet.rows.push(Array(sheet.col_count || 1).fill(''));
          }
          while (sheet.rows[row].length <= col) {
            sheet.rows[row].push('');
          }
          sheet.rows[row][col] = String(newVal ?? '');
        }
      }
      sheet.row_count = sheet.rows.length;
      const maxCols = sheet.rows.reduce((m, r) => Math.max(m, r.length), 0);
      sheet.col_count = maxCols;
      for (const r of sheet.rows) {
        while (r.length < maxCols) r.push('');
      }
      return next;
    });
  }, [activeSheet]);
  const handleAddRow = useCallback(() => {
    setSheets(prev => {
      const next = prev.map(s => ({
        ...s,
        rows: s.rows.map(r => [...r])
      }));
      const sheet = next[activeSheet];
      const emptyRow = Array(sheet.col_count || 1).fill('');
      sheet.rows.push(emptyRow);
      sheet.row_count = sheet.rows.length;
      setUnsaved(true);
      return next;
    });
  }, [activeSheet]);
  const handleDeleteRow = useCallback(() => {
    setSheets(prev => {
      const next = prev.map(s => ({
        ...s,
        rows: s.rows.map(r => [...r])
      }));
      const sheet = next[activeSheet];
      if (sheet.rows.length <= 1) return prev;
      sheet.rows.pop();
      sheet.row_count = sheet.rows.length;
      setUnsaved(true);
      return next;
    });
  }, [activeSheet]);
  const handleAddCol = useCallback(() => {
    setSheets(prev => {
      const next = prev.map(s => ({
        ...s,
        rows: s.rows.map(r => [...r])
      }));
      const sheet = next[activeSheet];
      for (const row of sheet.rows) {
        row.push('');
      }
      sheet.col_count = (sheet.col_count || 0) + 1;
      setUnsaved(true);
      return next;
    });
  }, [activeSheet]);
  const handleDeleteCol = useCallback(() => {
    setSheets(prev => {
      const next = prev.map(s => ({
        ...s,
        rows: s.rows.map(r => [...r])
      }));
      const sheet = next[activeSheet];
      if (sheet.col_count <= 1) return prev;
      for (const row of sheet.rows) {
        row.pop();
      }
      sheet.col_count = (sheet.col_count || 0) - 1;
      setUnsaved(true);
      return next;
    });
  }, [activeSheet]);
  const handleExportCsv = useCallback(async () => {
    try {
      const {
        ipc
      } = await import('@/lib/ipc');
      const res = await ipc.invoke<any>('tableedit_export_csv', {
        path: filePath,
        ext: fileExt,
        rows: sheetsRef.current[activeSheet]?.rows || [],
        sheet_name: sheetsRef.current[activeSheet]?.name || ''
      });
      if (res.code === 0) {
        showStatus('success', t("components.TableEditor.k2", {
          data: res.data
        }));
      } else {
        showStatus('error', res.message || t("components.TableEditor.k3"));
      }
    } catch (e: any) {
      showStatus('error', e?.toString() || t("components.TableEditor.k3"));
    }
  }, [filePath, fileExt, activeSheet, showStatus]);
  if (!currentSheet) {
    return <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      color: '#888'
    }}>
        {t("components.TableEditor.k4")}
      </div>;
  }
  return <div style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    background: '#0a0a10'
  }}>
      <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '8px 16px',
      borderBottom: '1px solid rgba(255,255,255,0.08)',
      background: '#0d0d14',
      flexShrink: 0
    }}>
        <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: '12px'
      }}>
          <span style={{
          color: '#888',
          fontSize: '13px'
        }}>
            {t("components.TableEditor.k5")} <strong style={{
            color: '#ccc'
          }}>{fileName}</strong>
          </span>
          <span style={{
          color: '#555',
          fontSize: '12px'
        }}>
            {currentSheet.row_count} {t("components.TableEditor.k6")} {currentSheet.col_count} {t("components.TableEditor.k7")}
          </span>
          {unsaved && <span style={{
          color: '#f0a030',
          fontSize: '12px',
          background: 'rgba(240,160,48,0.1)',
          padding: '2px 8px',
          borderRadius: '4px'
        }}>
              {t("components.AudioEditor.k5")}
            </span>}
        </div>
        <div style={{
        display: 'flex',
        gap: '8px'
      }}>
          <button onClick={handleExportCsv} title={t("components.TableEditor.k8")} style={{
          padding: '6px 12px',
          borderRadius: '4px',
          border: '1px solid rgba(255,255,255,0.12)',
          background: 'transparent',
          color: '#aaa',
          fontSize: '12px',
          cursor: 'pointer'
        }}>
            {t("components.TableEditor.k9")}
          </button>
          <button onClick={handleSave} disabled={isSaving} style={{
          padding: '6px 16px',
          borderRadius: '4px',
          border: 'none',
          background: isSaving ? '#555' : '#B026FF',
          color: '#fff',
          fontSize: '13px',
          cursor: isSaving ? 'not-allowed' : 'pointer'
        }}>
            {isSaving ? t("components.AudioEditor.k6") : t("components.TableEditor.k10")}
          </button>
          <button onClick={handleClose} style={{
          padding: '6px 16px',
          borderRadius: '4px',
          border: '1px solid rgba(255,255,255,0.12)',
          background: 'transparent',
          color: '#aaa',
          fontSize: '13px',
          cursor: 'pointer'
        }}>
            {t("components.FloatingXin.k28")}
          </button>
        </div>
      </div>

      {sheets.length > 1 && <div style={{
      display: 'flex',
      gap: '2px',
      padding: '4px 16px',
      background: '#0d0d14',
      borderBottom: '1px solid rgba(255,255,255,0.06)',
      flexShrink: 0
    }}>
          {sheets.map((s, i) => <button key={i} onClick={() => setActiveSheet(i)} style={{
        padding: '4px 14px',
        borderRadius: '4px 4px 0 0',
        border: 'none',
        background: i === activeSheet ? '#B026FF' : 'rgba(255,255,255,0.06)',
        color: i === activeSheet ? '#fff' : '#888',
        fontSize: '12px',
        cursor: 'pointer'
      }}>
              {s.name}
            </button>)}
        </div>}

      <div style={{
      display: 'flex',
      gap: '4px',
      padding: '6px 16px',
      background: '#0a0a10',
      borderBottom: '1px solid rgba(255,255,255,0.06)',
      flexShrink: 0
    }}>
        <button onClick={handleAddRow} style={{
        padding: '3px 10px',
        borderRadius: '3px',
        border: '1px solid rgba(255,255,255,0.1)',
        background: 'transparent',
        color: '#888',
        fontSize: '11px',
        cursor: 'pointer'
      }}>
          {t("components.TableEditor.k11")}
        </button>
        <button onClick={handleDeleteRow} style={{
        padding: '3px 10px',
        borderRadius: '3px',
        border: '1px solid rgba(255,255,255,0.1)',
        background: 'transparent',
        color: '#888',
        fontSize: '11px',
        cursor: 'pointer'
      }}>
          {t("components.TableEditor.k12")}
        </button>
        <button onClick={handleAddCol} style={{
        padding: '3px 10px',
        borderRadius: '3px',
        border: '1px solid rgba(255,255,255,0.1)',
        background: 'transparent',
        color: '#888',
        fontSize: '11px',
        cursor: 'pointer'
      }}>
          {t("components.TableEditor.k13")}
        </button>
        <button onClick={handleDeleteCol} style={{
        padding: '3px 10px',
        borderRadius: '3px',
        border: '1px solid rgba(255,255,255,0.1)',
        background: 'transparent',
        color: '#888',
        fontSize: '11px',
        cursor: 'pointer'
      }}>
          {t("components.TableEditor.k14")}
        </button>
      </div>

      <div style={{
      flex: 1,
      minHeight: 0,
      overflow: 'hidden'
    }}>
        <HotTable data={currentSheet.rows} colHeaders={true} rowHeaders={true} width="100%" height="100%" licenseKey="non-commercial-and-evaluation" afterChange={handleAfterChange} afterCreateRow={handleChange} afterCreateCol={handleChange} afterRemoveRow={(_i, _a, _r, s) => {
        if (s !== 'loadData') setUnsaved(true);
      }} afterRemoveCol={(_i, _a, _c, s) => {
        if (s !== 'loadData') setUnsaved(true);
      }} stretchH="all" contextMenu={true} manualRowResize={true} manualColumnResize={true} minRows={20} minCols={10} minSpareRows={1} />
      </div>
    </div>;
}