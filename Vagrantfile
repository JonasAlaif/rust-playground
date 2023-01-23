# -*- mode: ruby -*-
# vi: set ft=ruby :

require "getoptlong"

# Configure multiple machines
Vagrant.configure("2") do |config|
    runner_name = "russol-playground"
    config.vm.define runner_name do |node|
        # Set hostname
        node.vm.hostname = runner_name

        # Set OS
        node.vm.box = "ubuntu/focal64"

        # Configure VM resource limits
        node.vm.provider "virtualbox" do |vb|
            vb.memory = 16384
            vb.cpus = 16
        end

        # Install Docker
        node.vm.provision :docker

        # Install yarn
        node.vm.provision :shell, name: "Install Yarn", inline: <<-SHELL
            apt-get update
            apt-get install -y yarn
        SHELL

        # Install rustup
        node.vm.provision :shell, name: "Install rustup", privileged: false, inline: <<-SHELL
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        SHELL

        # Run playground
        node.vm.provision :shell, name: "Run playground", privileged: false, inline: <<-SHELL
            # Build frontend
            cd /vagrant/ui/frontend
            yarn run build

            # Build docker images
            cd /vagrant/compiler
            ./fetch.sh
            ./build_russol.sh

            # Run backend
            cd /vagrant/ui
            cargo run --release
        SHELL
    end
end
