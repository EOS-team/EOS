//
//  TestView.swift
//  KeyPointCenter
//
//  Created by 赵天禹 on 2023/8/8.
//

import SwiftUI
import UIKit

struct TestView: View {
    @State private var keyPoints = [KeyPoint]()
    
    var body: some View {
        List(keyPoints, id: \.self) { keyPoint in
            Text("\(keyPoint.pos3D.x), \(keyPoint.pos3D.y), \(keyPoint.pos3D.z), \(keyPoint.score3D)")
        }
        .onAppear(perform: {
            loadFromFile()
        })
        .navigationBarTitle("测试数据", displayMode: .inline)
    }
    
    func loadFromFile() {
        keyPoints = KeyPoint.testData()
    }
}

struct TestView_Previews: PreviewProvider {
    static var previews: some View {
        TestView()
    }
}
