using System.IO;
using NUnit.Framework;
using UnityEditor.SettingsManagement;

namespace UnityEngine.SettingsManagement.EditorTests
{
	class MultipleProjectSettings
	{
		const string k_MultipleSettingFilesPackageName = "com.unity.settings-manager.tests";
		const string k_SettingsFileA = "FileA";
		const string k_SettingsFileB = "FileB";

		[TearDown]
		public void Teardown()
		{
			var expectedPath = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName);
			var expectedPathA = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName, k_SettingsFileA);
			var expectedPathB = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName, k_SettingsFileB);

			if(File.Exists(expectedPath))
				File.Delete(expectedPath);

			if(File.Exists(expectedPathA))
				File.Delete(expectedPathA);

			if(File.Exists(expectedPathB))
				File.Delete(expectedPathB);
		}

		[Test]
		public void NewSettingsInstance_CreatesUserAndProjectSettings()
		{
			var expectedPath = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName);

			Assume.That(File.Exists(expectedPath), Is.False);

			var settings = new Settings(k_MultipleSettingFilesPackageName);
			settings.Save();

			Assert.That(File.Exists(expectedPath), Is.True);
		}

		[Test]
		public void NewSettingsInstance_SupportsMultipleProjectRepositories()
		{
			var expectedPathA = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName, "FileA");
			var expectedPathB = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName, "FileB");

			Assume.That(File.Exists(expectedPathA), Is.False);
			Assume.That(File.Exists(expectedPathB), Is.False);

			var settings = new Settings(new ISettingsRepository[]
			{
				new PackageSettingsRepository(k_MultipleSettingFilesPackageName, "FileA"),
				new PackageSettingsRepository(k_MultipleSettingFilesPackageName, "FileB")
			});

			settings.Save();

			Assert.That(File.Exists(expectedPathA), Is.True);
			Assert.That(File.Exists(expectedPathB), Is.True);
		}

		[Test]
		public void MultipleNamedProjectSettings_StoreSettingsSeparately()
		{
			var expectedPathA = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName, "FileA");
			var expectedPathB = PackageSettingsRepository.GetSettingsPath(k_MultipleSettingFilesPackageName, "FileB");

			Assume.That(File.Exists(expectedPathA), Is.False);
			Assume.That(File.Exists(expectedPathB), Is.False);

			var settings = new Settings(new ISettingsRepository[]
			{
				new PackageSettingsRepository(k_MultipleSettingFilesPackageName, "FileA"),
				new PackageSettingsRepository(k_MultipleSettingFilesPackageName, "FileB")
			});

			settings.Set<int>("value_a", 32, "FileA");
			settings.Set<int>("value_a", 64, "FileB");

			Assert.That(settings.Get<int>("value_a", "FileA"), Is.EqualTo(32));
			Assert.That(settings.Get<int>("value_a", "FileB"), Is.EqualTo(64));
		}
	}
}
