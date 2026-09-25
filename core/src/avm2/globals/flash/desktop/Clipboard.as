package flash.desktop {
    import __ruffle__.stub_method;

    import flash.system.System;

    public class Clipboard {
        private static var _generalClipboard = new Clipboard();

        public static function get generalClipboard():Clipboard {
            return Clipboard._generalClipboard;
        }

        function Clipboard() {
            // TODO: This should only be callable in AIR
        }

        public function get formats():Array {
            return hasText() ? [ClipboardFormats.TEXT_FORMAT] : [];
        }

        public function clear():void {
            System.setClipboard("");
        }

        public function clearData(format:String):void {
            if (format == ClipboardFormats.TEXT_FORMAT) {
                clear();
            }
        }

        public function getData(
            format:String,
            transferMode:String = ClipboardTransferMode.ORIGINAL_PREFERRED
        ):Object {
            if (format == ClipboardFormats.TEXT_FORMAT && hasText()) {
                return getText();
            }
            return null;
        }

        public function hasFormat(format:String):Boolean {
            return format == ClipboardFormats.TEXT_FORMAT && hasText();
        }

        private native function getText():String;
        private native function hasText():Boolean;

        public function setData(format:String, data:Object, serializable:Boolean = true):Boolean {
            if (format == ClipboardFormats.TEXT_FORMAT) {
                System.setClipboard(data);
                return true;
            }
            stub_method("flash.desktop.Clipboard", "setData");
            return false;
        }

        public function setDataHandler(format:String, handler:Function, serializable:Boolean = true):Boolean {
            stub_method("flash.desktop.Clipboard", "setDataHandler");
            return false;
        }
    }
}
