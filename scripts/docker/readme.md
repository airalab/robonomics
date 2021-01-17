== Robonomics Docker

=== OPTION1: Start a Robonomics docker container
Run the following command:
        docker run -d -P --name robonomics robonomics:latest

=== OPTION2: Start using docker-compose
Run the following command:
	docker-compose up -d

=== To stop docker-compose container
Run the following command:
	docker-compose stop

=== To remove docker-compose container
Run the following command:
	docker-compose rm

=== Building the image
To build your own image from the source, you can run the following command:
./build.sh
Note    Building the image takes a while. Count at least 30min on a good machine.
