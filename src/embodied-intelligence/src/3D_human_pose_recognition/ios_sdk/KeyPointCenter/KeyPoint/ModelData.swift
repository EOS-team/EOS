//
//  ModelData.swift
//  KeyPointCenter
//
//  Created by ZhaoTianyu on 2024/8/9.
//

import Foundation

final class ModelData {
    var humanBodyBones: [HumanBodyBone] = load("HumanBodyBones.json")
    
    func finaBone(with name: HumanBodyBone.Name) -> HumanBodyBone? {
        humanBodyBones.filter { bone in
            bone.name == name
        }.first
    }
    
    func getBoneTransform(_ name: HumanBodyBone.Name) -> Transform {
        guard let bodyBone = finaBone(with: name) else {
            fatalError("匹配到了错误的关节名称，请检查代码")
        }
        
        let transform = Transform()
        transform.location = Vector3(
            bodyBone.location.x,
            bodyBone.location.y,
            bodyBone.location.z
        )
        transform.localLocation = Vector3(
            bodyBone.localLocation.x,
            bodyBone.localLocation.y,
            bodyBone.localLocation.z
        )
        transform.rotation = Quaternion(
            ix: bodyBone.rotation.x,
            iy: bodyBone.rotation.y,
            iz: bodyBone.rotation.z,
            r: bodyBone.rotation.w
        )
        transform.localRotation = Quaternion(
            ix: bodyBone.localRotation.x,
            iy: bodyBone.localRotation.y,
            iz: bodyBone.localRotation.z,
            r: bodyBone.localRotation.w
        )
        return transform
    }
}

func load<T: Decodable>(_ filename: String) -> T {
    let data: Data
    
    guard let file = Bundle.main.url(forResource: filename, withExtension: nil) else {
        fatalError("Couldn't find \(filename) in main bundle.")
    }
    
    do {
        data = try Data(contentsOf: file)
    } catch {
        fatalError("Couldn't load \(filename) from main bundle:\n\(error)")
    }
    
    do {
        let decoder = JSONDecoder()
        return try decoder.decode(T.self, from: data)
    } catch {
        fatalError("Couldn't parse \(filename) as \(T.self):\n\(error)")
    }
}
