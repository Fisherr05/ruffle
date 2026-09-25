package {
    import flash.display.MovieClip;
    import flash.text.engine.ElementFormat;
    import flash.text.engine.TextBlock;
    import flash.text.engine.TextElement;

    public class Test extends MovieClip {
        public function Test() {
            var element:TextElement = new TextElement("a\u0301b");
            element.elementFormat = new ElementFormat();
            var block:TextBlock = new TextBlock(element);
            block.createTextLine();
            trace(block.findNextAtomBoundary(0));
            trace(block.findNextAtomBoundary(1));
            trace(block.findNextAtomBoundary(2));
            trace(block.findPreviousAtomBoundary(3));
            trace(block.findPreviousAtomBoundary(2));
            trace(block.findPreviousAtomBoundary(1));
        }
    }
}
