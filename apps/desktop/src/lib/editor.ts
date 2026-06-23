import { EditorState } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';
import 'prosemirror-view/style/prosemirror.css';
import {
  documentToEditorDoc,
  editorDocToWordCoreBlocks,
  type DocumentState,
  type EditorDoc,
  type EditorProjectedChange
} from './documentProjection';
import { supportedSchema } from './editorSchema';

export function createEditor(
  host: HTMLElement,
  document: DocumentState,
  onChange: (change: EditorProjectedChange) => void,
  options: { editable: boolean } = { editable: true }
): EditorView {
  const state = EditorState.create({
    doc: supportedSchema.nodeFromJSON(documentToEditorDoc(document))
  });

  const view = new EditorView(host, {
    state,
    editable: () => options.editable,
    dispatchTransaction(transaction) {
      const nextState = view.state.apply(transaction);
      view.updateState(nextState);
      onChange({
        text: view.state.doc.textContent,
        blocks: editorDocToWordCoreBlocks(view.state.doc.toJSON() as EditorDoc)
      });
    }
  });

  return view;
}
