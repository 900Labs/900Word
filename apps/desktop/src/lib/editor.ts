import { schema } from 'prosemirror-schema-basic';
import { EditorState } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';
import 'prosemirror-view/style/prosemirror.css';

export function createEditor(
  host: HTMLElement,
  initialText: string,
  onChange: (text: string) => void
): EditorView {
  const paragraph = schema.nodes.paragraph.create(null, initialText ? schema.text(initialText) : null);
  const state = EditorState.create({
    doc: schema.nodes.doc.create(null, [paragraph])
  });

  const view = new EditorView(host, {
    state,
    dispatchTransaction(transaction) {
      const nextState = view.state.apply(transaction);
      view.updateState(nextState);
      onChange(view.state.doc.textContent);
    }
  });

  return view;
}
