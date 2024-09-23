//
//  AvatarBonePoint.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/7.
//

import Foundation
import simd

class AvatarBonePoint: CustomStringConvertible {
    var description: String {
        let transformJson = transform?.description ?? "{}"
        return "{\n\"pos3D\": \(pos3D.description()), \"score3D\": \(score3D), \"transform\": \(transformJson), \"initLocalRotation\": \(initLocalRotation.description()), \"initRotation\": \(initRotation.description()), \"inverse\": \(inverse.description()), \"inverseRotation\": \(inverseRotation.description()), \"calcuRotation\": \(calcuRotation.description())}"
    }
    
    var idx: Int?
    
    var parent: AvatarBonePoint?
    var child: AvatarBonePoint?
    var child1: AvatarBonePoint?
    var child2: AvatarBonePoint?
    var enabled = true
    var lock = false
    var visibled = false
    var error: Int = 0
    var pos3D: Vector3 = .zero
    var score3D: Float = 0
    var transform: Transform? {
        didSet {
            makeBlock()
        }
    }
    var initLocalRotation: Quaternion = Quaternion(vector: simd_float4.zero)
    var initRotation: Quaternion = Quaternion(vector: simd_float4.zero)
    var inverse: Quaternion = Quaternion(vector: simd_float4.zero)
    var inverseRotation: Quaternion = Quaternion(vector: simd_float4.zero)
    var calcuRotation: Quaternion = Quaternion(vector: simd_float4.zero)
    
    init(idx: Int? = nil,
         parent: AvatarBonePoint? = nil,
         child: AvatarBonePoint? = nil,
         enabled: Bool = true,
         lock: Bool = false,
         visibled: Bool = false,
         error: Int = 0,
         pos3D: Vector3 = .zero,
         score3D: Float = 0,
         transform: Transform? = nil,
         initLocalRotation: Quaternion = Quaternion(vector: simd_float4.zero),
         initRotation: Quaternion = Quaternion(vector: simd_float4.zero),
         inverse: Quaternion = Quaternion(vector: simd_float4.zero),
         inverseRotation: Quaternion = Quaternion(vector: simd_float4.zero),
         calcuRotation: Quaternion = Quaternion(vector: simd_float4.zero)
    ) {
        self.idx = idx
        self.parent = parent
        self.child = child
        self.enabled = enabled
        self.lock = lock
        self.visibled = visibled
        self.error = error
        self.pos3D = pos3D
        self.score3D = score3D
        self.transform = transform
        self.initLocalRotation = initLocalRotation
        self.initRotation = initRotation
        self.inverse = inverse
        self.inverseRotation = inverseRotation
        self.calcuRotation = calcuRotation
        makeBlock()
    }
}

extension AvatarBonePoint {
    func makeBlock() {
        transform?.onRotation = { [weak self] in
            self?.correctChild()
        }
    }
    
    // Quaternion tmp_rot = point.Transform.rotation * Quaternion.Inverse(point.InitRotation) * this.avatarBonePoint[25].InitRotation;
    func correctChild() {
        guard let transform = self.transform else {
            return
        }
        
        if let idx = idx {
            if idx == 26 || idx == 25 {
                print("【DEBUG - rotation】idx-\(idx): \(transform.rotation)")
            }
        }
        
//        if let child = child, let rotation = child.transform?.rotation  {
//            if let idx = idx, idx == 26 {
//                print("26transform.rotation:\(transform.rotation)")
//                print("initRotation.inverse:\(initRotation.inverse)")
//                print("25rotation:\(rotation)")
//            }
//            child.transform?.rotation = transform.rotation * initRotation.inverse * rotation
//        }
//        if let child = child1, let rotation = child.transform?.rotation {
//            child.transform?.rotation = transform.rotation * initRotation.inverse * rotation
//        }
//        if let child = child2, let rotation = child.transform?.rotation {
//            child.transform?.rotation = transform.rotation * initRotation.inverse * rotation
//        }
        
        if let child = child  {
            if let idx = idx, idx == 26 {
                print("26transform.rotation:\(transform.rotation)")
                print("initRotation.inverse:\(initRotation.inverse)")
                print("child.initRotation:\(child.initRotation)")
            }
            child.transform?.rotation = transform.rotation * initRotation.inverse * child.initRotation
        }
        if let child = child1 {
            child.transform?.rotation = transform.rotation * initRotation.inverse * child.initRotation
        }
        if let child = child2 {
            child.transform?.rotation = transform.rotation * initRotation.inverse * child.initRotation
        }
    }
}
