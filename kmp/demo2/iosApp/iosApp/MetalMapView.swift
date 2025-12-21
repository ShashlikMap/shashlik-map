import SwiftUI
import UIKit
import Metal
import ComposeApp

// SwiftUI wrapper hosting the Metal-backed UIView and driving the render loop.
struct MetalMapContainer: UIViewRepresentable {
    func makeUIView(context: Context) -> MetalMapUIView {
        MetalMapUIView(frame: .zero)
    }
    func updateUIView(_ uiView: MetalMapUIView, context: Context) {}
}

final class MetalMapUIView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }

    private var displayLink: CADisplayLink?
    private(set) var shashlikMapApi: ShashlikMapApi?
    
    override init(frame: CGRect) {
        super.init(frame: frame)
        commonInit()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        commonInit()
    }

    private func commonInit() {
        isOpaque = true
        contentScaleFactor = UIScreen.main.scale
        backgroundColor = .black
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil {
            displayLink?.invalidate()
            displayLink = nil
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        // Defer creation until we have a concrete (non-zero) size; ensures wgpu surface config correct.
        if bounds.width > 0, bounds.height > 0 {
            initializeApiIfNeeded()
            shashlikMapApi?.resize(width: UInt32(bounds.width), height: UInt32(bounds.height))
        }
    }

    private func initializeApiIfNeeded() {
        guard shashlikMapApi == nil else { return }
        let viewPtr = Unmanaged.passUnretained(self).toOpaque()
        let layerPtr = Unmanaged.passUnretained(self.layer).toOpaque()

        shashlikMapApi = MainViewControllerKt.createShashlikMapApiForIos(view: UInt64(UInt(bitPattern: viewPtr)), metalLayer: UInt64(UInt(bitPattern: layerPtr)))
        
        startRendering()
    }

    private func startRendering() {
        displayLink = CADisplayLink(target: self, selector: #selector(frameTick))
        displayLink?.preferredFrameRateRange = CAFrameRateRange(minimum: 30, maximum: 60, preferred: 60)
        displayLink?.add(to: .main, forMode: .default)
    }

    @objc private func frameTick() {
        shashlikMapApi?.render()
    }
}

// Convenience SwiftUI view combining map and a control button.
struct MapWithControlsView: View {
    // Keep a weak link to underlying view to trigger input.
    @State private var metalViewRef: MetalMapUIView?

    var body: some View {
        MetalMapContainer()
            .background(GeometryReader { proxy in
                Color.clear.onAppear {
                    // Traverse view hierarchy to find MetalMapUIView.
                    if let uiView = findMetalView(in: UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.keyWindow }.first) {
                        metalViewRef = uiView
                    }
                }
            })
        .ignoresSafeArea()
    }

    private func findMetalView(in root: UIView?) -> MetalMapUIView? {
        guard let root else { return nil }
        if let mv = root as? MetalMapUIView { return mv }
        for sub in root.subviews {
            if let mv = findMetalView(in: sub) { return mv }
        }
        return nil
    }
}

#Preview {
    MapWithControlsView()
}
