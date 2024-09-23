//
//  KeyPointCenterApp.swift
//  KeyPointCenter
//
//  Created by 赵天禹 on 2023/8/8.
//

import SwiftUI

class AppDelegate: NSObject, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey : Any]? = nil) -> Bool {
        
        return true
    }
}

@main
struct KeyPointCenterApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .onOpenURL(perform: { url in
                    print("------ URL：")
                    print(url)
                    print("------ URL End ------")
                })
        }
    }
}
