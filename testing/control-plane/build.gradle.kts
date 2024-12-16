import com.bmuschko.gradle.docker.tasks.image.DockerBuildImage
import com.github.jengelman.gradle.plugins.shadow.ShadowJavaPlugin

plugins {
    `java-library`
    id("application")
    alias(libs.plugins.shadow)
    alias(libs.plugins.docker)
}

repositories {
    mavenCentral()
}

dependencies {
    runtimeOnly(libs.edc.bom.controlplane)
    runtimeOnly(libs.edc.iam.mock)
    runtimeOnly(libs.edc.cp.api.configuration)
    runtimeOnly(libs.edc.dp.selector.api)
    runtimeOnly(libs.edc.dp.signaling)
    implementation(libs.edc.spi.boot)
}

application {
    mainClass.set("org.eclipse.edc.boot.system.runtime.BaseRuntime")
}

tasks.withType<com.github.jengelman.gradle.plugins.shadow.tasks.ShadowJar> {
    dependsOn("distTar", "distZip")
    mergeServiceFiles()
    archiveFileName.set("control-plane.jar")
}

//actually apply the plugin to the (sub-)project
apply(plugin = "com.bmuschko.docker-remote-api")
// configure the "dockerize" task
val dockerTask: DockerBuildImage = tasks.create("dockerize", DockerBuildImage::class) {
    val dockerContextDir = project.projectDir
    dockerFile.set(file("$dockerContextDir/src/main/docker/Dockerfile"))
    images.add("${project.name}:${project.version}")
    images.add("${project.name}:latest")
    // specify platform with the -Dplatform flag:
    if (System.getProperty("platform") != null)
        platform.set(System.getProperty("platform"))
    buildArgs.put("JAR", "build/libs/${project.name}.jar")
    inputDir.set(file(dockerContextDir))
}
// make sure  always runs after "dockerize" and after "copyOtel"
dockerTask.dependsOn(tasks.named(ShadowJavaPlugin.SHADOW_JAR_TASK_NAME))
