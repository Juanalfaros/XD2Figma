import AppKit

let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let version = CommandLine.arguments[2]
let appPath = CommandLine.arguments[3]
let canvas = NSSize(width: 760, height: 500)
func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: 1)
}
func text(_ string: String, _ x: CGFloat, _ y: CGFloat, _ width: CGFloat, _ size: CGFloat, _ weight: NSFont.Weight, _ hex: UInt32, centered: Bool = false) {
    let p = NSMutableParagraphStyle(); p.alignment = centered ? .center : .left
    let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: size, weight: weight), .foregroundColor: color(hex), .paragraphStyle: p]
    (string as NSString).draw(in: NSRect(x: x, y: y, width: width, height: size * 1.6), withAttributes: attrs)
}
func render(scale: Int, preview: Bool) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 760 * scale, pixelsHigh: 500 * scale, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = canvas
    let bitmapContext = NSGraphicsContext(bitmapImageRep: rep)!
    let context = NSGraphicsContext(cgContext: bitmapContext.cgContext, flipped: true)
    NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = context
    let cg = context.cgContext; cg.translateBy(x: 0, y: 500); cg.scaleBy(x: 1, y: -1)
    color(0x171717).setFill(); NSRect(origin: .zero, size: canvas).fill()
    text("XD2Figma", 48, 34, 500, 30, .semibold, 0xF2F2F2)
    text("Tu diseño, en su próximo formato.", 48, 78, 500, 14, .regular, 0xA3A3A3)
    text("macOS · v\(version)", 530, 45, 182, 12, .medium, 0xA3A3A3, centered: true)
    color(0xF5F5F3).setFill(); NSBezierPath(roundedRect: NSRect(x: 48, y: 138, width: 664, height: 214), xRadius: 18, yRadius: 18).fill()
    // Real Finder icons occupy (212, 226) and (548, 226); keep the artwork clear of their hit targets.
    let arrow = NSBezierPath(); arrow.move(to: NSPoint(x: 337, y: 224)); arrow.line(to: NSPoint(x: 423, y: 224)); arrow.move(to: NSPoint(x: 409, y: 210)); arrow.line(to: NSPoint(x: 423, y: 224)); arrow.line(to: NSPoint(x: 409, y: 238)); arrow.lineWidth = 2.5; arrow.lineCapStyle = .round; arrow.lineJoinStyle = .round; color(0x858581).setStroke(); arrow.stroke()
    text("INSTALAR ES ASÍ DE SIMPLE", 48, 382, 664, 10, .semibold, 0xA3A3A3, centered: true)
    text("Arrastra XD2Figma a Aplicaciones", 40, 407, 680, 23, .semibold, 0xF2F2F2, centered: true)
    text("Después, abre la app desde tu carpeta Aplicaciones.", 48, 446, 664, 13, .regular, 0xA3A3A3, centered: true)
    if preview {
        for (path, x, title) in [(appPath, CGFloat(212), "XD2Figma"), ("/Applications", CGFloat(548), "Aplicaciones")] {
            let iconPath = path == "/Applications" ? "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/ApplicationsFolderIcon.icns" : path + "/Contents/Resources/icon.icns"
            let icon = NSImage(contentsOfFile: iconPath)!
            icon.draw(in: NSRect(x: x - 54, y: 172, width: 108, height: 108), from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)
            text(title, x - 100, 292, 200, 13, .medium, 0x242424, centered: true)
        }
    }
    NSGraphicsContext.restoreGraphicsState()
    return rep
}
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
let image = NSImage(size: canvas)
image.addRepresentation(render(scale: 1, preview: false)); image.addRepresentation(render(scale: 2, preview: false))
try image.tiffRepresentation!.write(to: output.appendingPathComponent("background.tiff"))
try render(scale: 2, preview: true).representation(using: .png, properties: [:])!.write(to: output.appendingPathComponent("preview.png"))
