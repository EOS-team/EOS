//
//  Transform.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/7.
//

import Foundation
import simd
import SceneKit

class Transform: CustomStringConvertible {
    typealias OnRotation = () -> Void
    var onRotation: OnRotation?
    
    var description: String {
        return "{\"location\": \(location.description()), \"localLocation\": \(localLocation.description()), \"rotation\": \(rotation.description()), \"localRotation\": \(localRotation.description())}"
    }
    
    var location: Vector3 = .zero
    var localLocation: Vector3 = .zero
    var rotation: Quaternion = .zero {
        didSet {
            onRotation?()
        }
    }
    var localRotation: Quaternion = .zero
    var position: Vector3 {
        get { return location }
        set { location = newValue }
    }
}
