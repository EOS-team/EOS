//
//  ResultView.swift
//  KeyPointCenter
//
//  Created by 赵天禹 on 2023/8/8.
//

import SwiftUI

struct ResultView: View {
    @State var count = 0
    var manager = MoveNetSample()
    
    var body: some View {
        Text("代码执行成功：结果已显示在控制台 \n avatarBonePoint.count = \(count)")
        .onAppear(perform: {
            loadFromFile()
        })
        .navigationBarTitle("执行测试数据", displayMode: .inline)
    }
    
    func loadFromFile() {
        manager.start()
        let keyPoints = KeyPoint.testData()
        manager.predictPose3D(points: keyPoints)
//        count = manager.avatarBonePoint.count
//        print("result:")
//        print(manager.avatarBonePoint)
    }
}

struct ResultView_Previews: PreviewProvider {
    static var previews: some View {
        ResultView()
    }
}
