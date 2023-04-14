# -*- mode: ruby -*-
# vi: set ft=ruby :

require "getoptlong"

# Install vagrant-disksize to allow resizing the vagrant box disk.
unless Vagrant.has_plugin?("vagrant-disksize")
    raise  Vagrant::Errors::VagrantError.new, "vagrant-disksize plugin is missing. Please install it using 'vagrant plugin install vagrant-disksize' and rerun 'vagrant up'"
end

# Configure
Vagrant.configure("2") do |config|
    config.vm.network "forwarded_port", guest: 5000, host: 5000
    config.disksize.size = '80GB'

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
            apt-get install -y build-essential libssl-dev pkg-config

            curl -fsSL https://deb.nodesource.com/setup_16.x | sudo -E bash -
            apt-get install -y nodejs

            curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
            echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list
            apt update
            apt install yarn
        SHELL

        # Install rustup
        node.vm.provision :shell, name: "Install rustup", privileged: false, inline: <<-SHELL
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        SHELL

        # Run playground
        node.vm.provision :shell, name: "Run playground", privileged: false, inline: <<-SHELL
            # Build frontend
            cd /vagrant/ui/frontend
            yarn
            yarn run build

            # Build docker images
            cd /vagrant/compiler
            ./build.sh

            # Run backend
            cd /vagrant/ui
            nohup cargo run --release &
        SHELL
    end
end
