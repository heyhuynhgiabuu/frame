#!/usr/bin/env swift

// GenerateWallpaperThumbnails.swift
// Creates thumbnail images for wallpaper presets

import Foundation
import CoreGraphics
import ImageIO

// MARK: - Color Definitions

struct ThumbnailSpec {
    let name: String
    let colors: [(r: CGFloat, g: CGFloat, b: CGFloat)]
    let style: Style
    
    enum Style {
        case gradient
        case solid
        case pattern
    }
}

let thumbnails: [ThumbnailSpec] = [
    // Gradients
    ThumbnailSpec(name: "sunset", colors: [(1.0, 0.6, 0.2), (1.0, 0.3, 0.5), (0.6, 0.2, 0.8)], style: .gradient),
    ThumbnailSpec(name: "ocean", colors: [(0.1, 0.4, 0.8), (0.2, 0.7, 0.9), (0.3, 0.8, 0.8)], style: .gradient),
    ThumbnailSpec(name: "forest", colors: [(0.2, 0.6, 0.3), (0.4, 0.8, 0.5), (0.3, 0.7, 0.6)], style: .gradient),
    ThumbnailSpec(name: "lavender", colors: [(0.5, 0.3, 0.8), (0.4, 0.4, 0.9), (0.3, 0.5, 0.9)], style: .gradient),
    ThumbnailSpec(name: "midnight", colors: [(0.1, 0.1, 0.15), (0.15, 0.1, 0.3), (0.1, 0.1, 0.15)], style: .gradient),
    
    // Solids
    ThumbnailSpec(name: "dark", colors: [(0.1, 0.1, 0.12)], style: .solid),
    ThumbnailSpec(name: "light", colors: [(0.95, 0.95, 0.97)], style: .solid),
    ThumbnailSpec(name: "warm", colors: [(0.98, 0.94, 0.88)], style: .solid),
    
    // Patterns (simulated with subtle gradients)
    ThumbnailSpec(name: "mesh", colors: [(0.15, 0.15, 0.2), (0.2, 0.15, 0.25), (0.15, 0.2, 0.22)], style: .pattern),
]

// MARK: - Image Generation

func createGradientThumbnail(spec: ThumbnailSpec, size: CGSize) -> CGImage? {
    let width = Int(size.width)
    let height = Int(size.height)
    
    guard let context = CGContext(
        data: nil,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        return nil
    }
    
    let colors = spec.colors.map { CGColor(red: $0.r, green: $0.g, blue: $0.b, alpha: 1.0) }
    
    switch spec.style {
    case .gradient:
        // Draw linear gradient
        let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                                  colors: colors as CFArray,
                                  locations: nil)!
        context.drawLinearGradient(gradient,
                                   start: CGPoint(x: 0, y: 0),
                                   end: CGPoint(x: CGFloat(width), y: CGFloat(height)),
                                   options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
        
    case .solid:
        // Fill with solid color
        if let color = colors.first {
            context.setFillColor(color)
            context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        }
        
    case .pattern:
        // Draw gradient with subtle variation
        let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                                  colors: colors as CFArray,
                                  locations: nil)!
        context.drawRadialGradient(gradient,
                                   startCenter: CGPoint(x: width/2, y: height/2),
                                   startRadius: 0,
                                   endCenter: CGPoint(x: width/2, y: height/2),
                                   endRadius: CGFloat(max(width, height)),
                                   options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
    }
    
    return context.makeImage()
}

func saveImage(_ image: CGImage, to path: String) -> Bool {
    guard let destination = CGImageDestinationCreateWithURL(
        URL(fileURLWithPath: path) as CFURL,
        kUTTypePNG,
        1,
        nil
    ) else {
        return false
    }
    
    CGImageDestinationAddImage(destination, image, nil)
    return CGImageDestinationFinalize(destination)
}

// MARK: - Main

let outputDir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."
let size = CGSize(width: 160, height: 160) // 80pt @2x

print("Generating wallpaper thumbnails...")
print("Output directory: \(outputDir)")
print("")

var successCount = 0
var failureCount = 0

for spec in thumbnails {
    if let image = createGradientThumbnail(spec: spec, size: size) {
        let outputPath = "\(outputDir)/\(spec.name).png"
        if saveImage(image, to: outputPath) {
            print("✓ Created \(spec.name).png")
            successCount += 1
        } else {
            print("✗ Failed to save \(spec.name).png")
            failureCount += 1
        }
    } else {
        print("✗ Failed to create \(spec.name)")
        failureCount += 1
    }
}

print("")
print("Done: \(successCount) created, \(failureCount) failed")
exit(failureCount > 0 ? 1 : 0)
