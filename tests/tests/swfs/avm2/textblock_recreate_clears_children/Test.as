package {
    import flash.display.MovieClip;
    import flash.display.Shape;
    import flash.text.engine.ElementFormat;
    import flash.text.engine.TextBlock;
    import flash.text.engine.TextElement;
    import flash.text.engine.TextLine;

    public class Test extends MovieClip {
        public function Test() {
            var content:TextElement = new TextElement("reused line");
            content.elementFormat = new ElementFormat();
            var block:TextBlock = new TextBlock(content);
            var line:TextLine = block.createTextLine(null, 1000);
            var decoration:Shape = new Shape();

            line.addChild(decoration);
            trace(line.numChildren);
            trace(decoration.parent === line);

            var recreated:TextLine = block.recreateTextLine(line, null, 1000);
            trace(recreated === line);
            trace(line.numChildren);
            trace(decoration.parent === null);
        }
    }
}
