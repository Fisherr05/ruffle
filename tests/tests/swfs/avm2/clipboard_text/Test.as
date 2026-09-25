package {
    import flash.desktop.Clipboard;
    import flash.desktop.ClipboardFormats;
    import flash.display.MovieClip;

    public class Test extends MovieClip {
        public function Test() {
            var clipboard:Clipboard = Clipboard.generalClipboard;
            clipboard.clear();
            trace(clipboard.hasFormat(ClipboardFormats.TEXT_FORMAT));
            trace(clipboard.setData(ClipboardFormats.TEXT_FORMAT, "copied text"));
            trace(clipboard.hasFormat(ClipboardFormats.TEXT_FORMAT));
            trace(clipboard.getData(ClipboardFormats.TEXT_FORMAT));
            trace(clipboard.formats.join(","));
            clipboard.clearData(ClipboardFormats.TEXT_FORMAT);
            trace(clipboard.hasFormat(ClipboardFormats.TEXT_FORMAT));
        }
    }
}
