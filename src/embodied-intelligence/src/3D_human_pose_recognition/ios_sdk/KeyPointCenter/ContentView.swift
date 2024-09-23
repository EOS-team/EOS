//
//  ContentView.swift
//  KeyPointCenter
//
//  Created by 赵天禹 on 2023/8/8.
//

import SwiftUI

struct ContentView: View {
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                NavigationLink(destination: TestView()) {
                    Text("测试数据展示")
                        .padding()
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(10)
                }
                
                NavigationLink(destination: ResultView()) {
                    Text("执行测试数据")
                        .padding()
                        .background(Color.green)
                        .foregroundColor(.white)
                        .cornerRadius(10)
                }

            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
