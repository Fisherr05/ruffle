package {
    import flash.display.Sprite;

    public class Test extends Sprite {
        public function Test() {
            // Flex TLF uses this expression to parse tabStops such as "S0 S50".
            var pattern:RegExp = /([sScCeEdD]?)([^| ]+)(|[^ ]*)?( |$)/g;
            var result:Object = pattern.exec("S0 S50 S100 S150 S200 S250");
            trace(result[1]);
            trace(result[2]);
            trace(result[3] === "");
        }
    }
}
